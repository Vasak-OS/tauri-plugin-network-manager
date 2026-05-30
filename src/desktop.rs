use std::collections::HashMap;
use std::sync::mpsc;
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use uuid::Uuid;
use zbus::names::InterfaceName;
use zbus::zvariant::Value;

use crate::error::Result;
use crate::models::*;
use crate::nm_helpers::NetworkManagerHelpers;

impl<R: Runtime> VSKNetworkManager<'static, R> {
    fn vpn_type_from_service_type(service_type: &str) -> VpnType {
        match service_type {
            "org.freedesktop.NetworkManager.openvpn" => VpnType::OpenVpn,
            "org.freedesktop.NetworkManager.wireguard" => VpnType::WireGuard,
            "org.freedesktop.NetworkManager.l2tp" => VpnType::L2tp,
            "org.freedesktop.NetworkManager.pptp" => VpnType::Pptp,
            "org.freedesktop.NetworkManager.sstp" => VpnType::Sstp,
            "org.freedesktop.NetworkManager.strongswan" => VpnType::Ikev2,
            "org.freedesktop.NetworkManager.fortisslvpn" => VpnType::Fortisslvpn,
            "org.freedesktop.NetworkManager.openconnect" => VpnType::OpenConnect,
            _ => VpnType::Generic,
        }
    }

    fn service_type_from_vpn_type(vpn_type: &VpnType) -> &'static str {
        match vpn_type {
            VpnType::OpenVpn => "org.freedesktop.NetworkManager.openvpn",
            VpnType::WireGuard => "org.freedesktop.NetworkManager.wireguard",
            VpnType::L2tp => "org.freedesktop.NetworkManager.l2tp",
            VpnType::Pptp => "org.freedesktop.NetworkManager.pptp",
            VpnType::Sstp => "org.freedesktop.NetworkManager.sstp",
            VpnType::Ikev2 => "org.freedesktop.NetworkManager.strongswan",
            VpnType::Fortisslvpn => "org.freedesktop.NetworkManager.fortisslvpn",
            VpnType::OpenConnect => "org.freedesktop.NetworkManager.openconnect",
            VpnType::Generic => "org.freedesktop.NetworkManager.vpnc",
        }
    }

    fn vpn_state_from_active_state(state: u32) -> VpnConnectionState {
        match state {
            1 => VpnConnectionState::Connecting,
            2 => VpnConnectionState::Connected,
            3 => VpnConnectionState::Disconnecting,
            4 => VpnConnectionState::Disconnected,
            _ => VpnConnectionState::Unknown,
        }
    }

    fn extract_string_from_dict(
        dict: &HashMap<String, zbus::zvariant::OwnedValue>,
        key: &str,
    ) -> Option<String> {
        let value = dict.get(key)?;
        let v: &zbus::zvariant::Value<'_> = value;
        v.downcast_ref::<String>().ok()
    }

    fn extract_bool_from_dict(
        dict: &HashMap<String, zbus::zvariant::OwnedValue>,
        key: &str,
    ) -> Option<bool> {
        let value = dict.get(key)?;
        let v: &zbus::zvariant::Value<'_> = value;
        v.downcast_ref::<bool>().ok()
    }

    fn string_map_from_section(
        settings: &HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>,
        section_name: &str,
    ) -> HashMap<String, String> {
        let mut out = HashMap::new();
        let section = match settings.get(section_name) {
            Some(v) => v,
            None => return out,
        };

        for (k, v) in section {
            let val: &zbus::zvariant::Value<'_> = v;
            if let Ok(s) = val.downcast_ref::<String>() {
                out.insert(k.clone(), s);
            }
        }
        out
    }

    fn list_connection_paths(&self) -> Result<Vec<zbus::zvariant::OwnedObjectPath>> {
        let settings_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager/Settings",
            "org.freedesktop.NetworkManager.Settings",
        )?;

        let connections: Vec<zbus::zvariant::OwnedObjectPath> =
            settings_proxy.call("ListConnections", &())?;
        Ok(connections)
    }

    fn get_connection_settings(
        &self,
        conn_path: &zbus::zvariant::OwnedObjectPath,
    ) -> Result<HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>> {
        let conn_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            conn_path.as_str(),
            "org.freedesktop.NetworkManager.Settings.Connection",
        )?;

        let settings: HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>> =
            conn_proxy.call("GetSettings", &())?;
        Ok(settings)
    }

    fn find_connection_path_by_uuid(
        &self,
        uuid: &str,
    ) -> Result<zbus::zvariant::OwnedObjectPath> {
        let connections = self.list_connection_paths()?;

        for conn_path in connections {
            let settings = self.get_connection_settings(&conn_path)?;
            let dict = match settings.get("connection") {
                Some(v) => v,
                None => continue,
            };

            if let Some(conn_uuid) = Self::extract_string_from_dict(&dict, "uuid") {
                if conn_uuid == uuid {
                    return Ok(conn_path);
                }
            }
        }

        Err(crate::error::NetworkError::VpnProfileNotFound(uuid.to_string()))
    }

    fn vpn_profile_from_settings(
        &self,
        settings: &HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>,
    ) -> Option<VpnProfile> {
        let connection_dict = settings.get("connection")?;

        let conn_type = Self::extract_string_from_dict(connection_dict, "type")?;
        if conn_type != "vpn" {
            return None;
        }

        let uuid = Self::extract_string_from_dict(connection_dict, "uuid")?;
        let id = Self::extract_string_from_dict(connection_dict, "id")
            .unwrap_or_else(|| uuid.clone());
        let interface_name = Self::extract_string_from_dict(connection_dict, "interface-name");
        let autoconnect = Self::extract_bool_from_dict(connection_dict, "autoconnect")
            .unwrap_or(false);

        let vpn_settings = Self::string_map_from_section(settings, "vpn");
        let service_type = vpn_settings
            .get("service-type")
            .map(|s| s.as_str())
            .unwrap_or("unknown");

        Some(VpnProfile {
            id,
            uuid,
            vpn_type: Self::vpn_type_from_service_type(service_type),
            interface_name,
            autoconnect,
            editable: true,
            last_error: None,
        })
    }

    /// Get WiFi icon based on signal strength
    fn get_wifi_icon(strength: u8) -> String {
        match strength {
            0..=25 => "network-wireless-signal-weak-symbolic".to_string(),
            26..=50 => "network-wireless-signal-ok-symbolic".to_string(),
            51..=75 => "network-wireless-signal-good-symbolic".to_string(),
            76..=100 => "network-wireless-signal-excellent-symbolic".to_string(),
            _ => "network-wireless-signal-none-symbolic".to_string(),
        }
    }

    fn get_wired_icon(is_connected: bool) -> String {
        if is_connected {
            "network-wired-symbolic".to_string()
        } else {
            "network-offline-symbolic".to_string()
        }
    }

    /// Create a new VSKNetworkManager instance
    pub async fn new(app: AppHandle<R>) -> Result<Self> {
        let connection = zbus::blocking::Connection::system()?;
        let proxy = zbus::blocking::fdo::PropertiesProxy::builder(&connection)
            .destination("org.freedesktop.NetworkManager")?
            .path("/org/freedesktop/NetworkManager")?
            .build()?;

        Ok(Self {
            connection,
            proxy,
            app,
        })
    }

    pub fn get_current_network_state(&self) -> Result<NetworkInfo> {
        // Get active connections
        let active_connections_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"),
            "ActiveConnections",
        )?;

        // If no active connections, return default
        match active_connections_variant.downcast_ref() {
            Ok(Value::Array(arr)) if !arr.is_empty() => {
                // Get the first active connection path
                match arr[0] {
                    zbus::zvariant::Value::ObjectPath(ref path) => {
                        // Get devices for this connection
                        // Crear un proxy de propiedades para obtener las propiedades
                        let properties_proxy =
                            zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                .destination("org.freedesktop.NetworkManager")?
                                .path(path)?
                                .build()?;

                        let devices_variant = properties_proxy.get(
                            InterfaceName::from_static_str_unchecked(
                                "org.freedesktop.NetworkManager.Connection.Active",
                            ),
                            "Devices",
                        )?;

                        // Get the first device (if available)
                        let device_path = match devices_variant.downcast_ref() {
                            Ok(Value::Array(device_arr)) if !device_arr.is_empty() => {
                                match device_arr[0] {
                                    zbus::zvariant::Value::ObjectPath(ref dev_path) => {
                                        dev_path.clone()
                                    }
                                    _ => return Ok(NetworkInfo::default()),
                                }
                            }
                            _ => return Ok(NetworkInfo::default()),
                        };

                        // Retrieve connection details
                        // Crear un proxy de propiedades para el dispositivo
                        let device_properties_proxy =
                            zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                .destination("org.freedesktop.NetworkManager")?
                                .path(&device_path)?
                                .build()?;

                        let connection_type = device_properties_proxy.get(
                            InterfaceName::from_static_str_unchecked(
                                "org.freedesktop.NetworkManager.Device",
                            ),
                            "DeviceType",
                        )?;

                        let state_variant = properties_proxy.get(
                            InterfaceName::from_static_str_unchecked(
                                "org.freedesktop.NetworkManager.Connection.Active",
                            ),
                            "State",
                        )?;

                        let is_connected = match state_variant.downcast_ref() {
                            Ok(zbus::zvariant::Value::U32(state)) => state == 2, // 2 = ACTIVATED
                            _ => false,
                        };

                        // Determine connection type
                        let connection_type_str = match connection_type.downcast_ref() {
                            Ok(zbus::zvariant::Value::U32(device_type)) => match device_type {
                                1 => "Ethernet".to_string(),
                                2 => "WiFi".to_string(),
                                _ => "Unknown".to_string(),
                            },
                            _ => "Unknown".to_string(),
                        };

                        // Default network info
                        let mut network_info = NetworkInfo {
                            name: "Unknown".to_string(),
                            ssid: "Unknown".to_string(),
                            connection_type: connection_type_str.clone(),
                            icon: "network-offline-symbolic".to_string(),
                            ip_address: "0.0.0.0".to_string(),
                            mac_address: "00:00:00:00:00:00".to_string(),
                            signal_strength: 0,
                            security_type: WiFiSecurityType::None,
                            is_connected: is_connected && NetworkManagerHelpers::has_internet_connectivity(&self.proxy)?,
                        };

                        let hw_address_variant = device_properties_proxy.get(
                            InterfaceName::from_static_str_unchecked(
                                "org.freedesktop.NetworkManager.Device",
                            ),
                            "HwAddress",
                        )?;

                        network_info.mac_address = match hw_address_variant.downcast_ref() {
                            Ok(zbus::zvariant::Value::Str(s)) => s.to_string(),
                            _ => "00:00:00:00:00:00".to_string(),
                        };

                        // For WiFi networks, get additional details
                        if connection_type_str == "WiFi" {
                            // Get active access point
                            // Crear un proxy de propiedades para el dispositivo inalámbrico
                            let wireless_properties_proxy =
                                zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                    .destination("org.freedesktop.NetworkManager")?
                                    .path(&device_path)?
                                    .build()?;

                            let active_ap_path = wireless_properties_proxy.get(
                                InterfaceName::from_static_str_unchecked(
                                    "org.freedesktop.NetworkManager.Device.Wireless",
                                ),
                                "ActiveAccessPoint",
                            )?;

                            if let Ok(zbus::zvariant::Value::ObjectPath(ap_path)) =
                                active_ap_path.downcast_ref()
                            {
                                let _ap_proxy = zbus::blocking::Proxy::new(
                                    &self.connection,
                                    "org.freedesktop.NetworkManager",
                                    &ap_path,
                                    "org.freedesktop.NetworkManager.AccessPoint",
                                )?;

                                // Get SSID
                                // Crear un proxy de propiedades para el punto de acceso
                                let ap_properties_proxy =
                                    zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                        .destination("org.freedesktop.NetworkManager")?
                                        .path(ap_path.as_str())?
                                        .build()?;

                                let ssid_variant = ap_properties_proxy.get(
                                    InterfaceName::from_static_str_unchecked(
                                        "org.freedesktop.NetworkManager.AccessPoint",
                                    ),
                                    "Ssid",
                                )?;

                                network_info.ssid = match ssid_variant.downcast_ref() {
                                    Ok(v) => NetworkManagerHelpers::ssid_from_value(&v),
                                    _ => "Unknown".to_string(),
                                };
                                network_info.name = network_info.ssid.clone();

                                // Get signal strength
                                let strength_variant = ap_properties_proxy.get(
                                    InterfaceName::from_static_str_unchecked(
                                        "org.freedesktop.NetworkManager.AccessPoint",
                                    ),
                                    "Strength",
                                )?;

                                network_info.signal_strength = match strength_variant.downcast_ref()
                                {
                                    Ok(zbus::zvariant::Value::U8(s)) => s,
                                    _ => 0,
                                };

                                // Update icon based on signal strength
                                network_info.icon =
                                    Self::get_wifi_icon(network_info.signal_strength);

                                // Determine security type using helper
                                network_info.security_type = NetworkManagerHelpers::detect_security_type(&ap_properties_proxy)?;
                            }
                        } else {
                            // This is a wired connection
                            network_info.icon = Self::get_wired_icon(network_info.is_connected);
                        }
                        // Get IP configuration
                        let ip4_config_path = device_properties_proxy.get(
                            InterfaceName::from_static_str_unchecked(
                                "org.freedesktop.NetworkManager.Device",
                            ),
                            "Ip4Config",
                        )?;

                        // Retrieve IP address if available
                        if let Ok(zbus::zvariant::Value::ObjectPath(config_path)) =
                            ip4_config_path.downcast_ref()
                        {
                            // Crear un proxy de propiedades para la configuración IP
                            let ip_config_properties_proxy =
                                zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                    .destination("org.freedesktop.NetworkManager")?
                                    .path(config_path)?
                                    .build()?;

                            let addresses_variant = ip_config_properties_proxy.get(
                                InterfaceName::from_static_str_unchecked(
                                    "org.freedesktop.NetworkManager.IP4Config",
                                ),
                                "Addresses",
                            )?;

                            if let Ok(Value::Array(addr_arr)) = addresses_variant.downcast_ref() {
                                if let Some(Value::Array(ip_tuple)) = addr_arr.first() {
                                    if ip_tuple.len() >= 1 {
                                        if let Value::U32(ip_int) = &ip_tuple[0] {
                                            use std::net::Ipv4Addr;
                                            network_info.ip_address =
                                                Ipv4Addr::from((*ip_int).to_be()).to_string();
                                        }
                                    }
                                }
                            }
                        }

                        Ok(network_info)
                    }
                    _ => Ok(NetworkInfo::default()),
                }
            }
            _ => Ok(NetworkInfo::default()),
        }
    }

    /// List available WiFi networks
    pub fn list_wifi_networks(&self) -> Result<Vec<NetworkInfo>> {
        // Get all devices
        let devices_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"),
            "Devices",
        )?;

        let mut networks = Vec::new();
        let current_network = self.get_current_network_state()?;

        if let Ok(zbus::zvariant::Value::Array(devices)) = devices_variant.downcast_ref() {
            // Iterate over devices in the array
            for device in devices.iter() {
                if let zbus::zvariant::Value::ObjectPath(ref device_path) = device {
                    // Create a device proxy
                    let device_props =
                        zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                            .destination("org.freedesktop.NetworkManager")?
                            .path(device_path)?
                            .build()?;

                    // Check if this is a wireless device
                    let device_type_variant = device_props.get(
                        InterfaceName::from_static_str_unchecked(
                            "org.freedesktop.NetworkManager.Device",
                        ),
                        "DeviceType",
                    )?;

                    // DeviceType 2 is WiFi
                    if let Ok(zbus::zvariant::Value::U32(device_type)) =
                        device_type_variant.downcast_ref()
                    {
                        if device_type == 2u32 {
                            let mac_address = match device_props
                                .get(
                                    InterfaceName::from_static_str_unchecked(
                                        "org.freedesktop.NetworkManager.Device",
                                    ),
                                    "HwAddress",
                                )?
                                .downcast_ref()
                            {
                                Ok(zbus::zvariant::Value::Str(s)) => s.to_string(),
                                _ => "00:00:00:00:00:00".to_string(),
                            };

                            // This is a WiFi device, get its access points
                            let wireless_props =
                                zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                    .destination("org.freedesktop.NetworkManager")?
                                    .path(device_path)?
                                    .build()?;

                            let access_points_variant = wireless_props.get(
                                InterfaceName::from_static_str_unchecked(
                                    "org.freedesktop.NetworkManager.Device.Wireless",
                                ),
                                "AccessPoints",
                            )?;

                            if let Ok(zbus::zvariant::Value::Array(aps)) =
                                access_points_variant.downcast_ref()
                            {
                                // Iterate over access points
                                for ap in aps.iter() {
                                    if let zbus::zvariant::Value::ObjectPath(ref ap_path) = ap {
                                        let ap_props = zbus::blocking::fdo::PropertiesProxy::builder(
                                            &self.connection,
                                        )
                                        .destination("org.freedesktop.NetworkManager")?
                                        .path(ap_path)?
                                        .build()?;

                                        // Obtener SSID
                                        let ssid_variant = ap_props.get(
                                            InterfaceName::from_static_str_unchecked(
                                                "org.freedesktop.NetworkManager.AccessPoint",
                                            ),
                                            "Ssid",
                                        )?;

                                        let ssid = match ssid_variant.downcast_ref() {
                                            Ok(v) => NetworkManagerHelpers::ssid_from_value(&v),
                                            _ => "Unknown".to_string(),
                                        };

                                        // Obtener fuerza de señal
                                        let strength_variant = ap_props.get(
                                            InterfaceName::from_static_str_unchecked(
                                                "org.freedesktop.NetworkManager.AccessPoint",
                                            ),
                                            "Strength",
                                        )?;

                                        let strength = match strength_variant.downcast_ref() {
                                            Ok(zbus::zvariant::Value::U8(s)) => s,
                                            _ => 0,
                                        };

                                        // Determine security type using helper
                                        let security_type = NetworkManagerHelpers::detect_security_type(&ap_props)?;

                                        let is_connected = current_network.ssid == ssid;

                                        let network_info = NetworkInfo {
                                            name: ssid.clone(),
                                            ssid,
                                            connection_type: "wifi".to_string(),
                                            icon: Self::get_wifi_icon(strength),
                                            ip_address: if is_connected {
                                                current_network.ip_address.clone()
                                            } else {
                                                "0.0.0.0".to_string()
                                            },
                                            mac_address: mac_address.clone(),
                                            signal_strength: strength,
                                            security_type,
                                            is_connected,
                                        };

                                        if !networks.iter().any(|n: &NetworkInfo| n.ssid == network_info.ssid) {
                                            networks.push(network_info);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort networks by signal strength (descending)
        networks.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));

        Ok(networks)
    }

    /// Request an explicit WiFi scan through NetworkManager and return a fresh list.
    pub fn rescan_wifi(&self) -> Result<Vec<NetworkInfo>> {
        let devices_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"),
            "Devices",
        )?;

        let mut wifi_device_found = false;
        let mut requested_scan = false;

        if let Ok(zbus::zvariant::Value::Array(devices)) = devices_variant.downcast_ref() {
            for device in devices.iter() {
                if let zbus::zvariant::Value::ObjectPath(ref device_path) = device {
                    let device_props = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                        .destination("org.freedesktop.NetworkManager")?
                        .path(device_path)?
                        .build()?;

                    let device_type_variant = device_props.get(
                        InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device"),
                        "DeviceType",
                    )?;

                    if let Ok(zbus::zvariant::Value::U32(device_type)) = device_type_variant.downcast_ref() {
                        if device_type == 2 {
                            wifi_device_found = true;

                            let wireless_proxy = zbus::blocking::Proxy::new(
                                &self.connection,
                                "org.freedesktop.NetworkManager",
                                device_path.as_str(),
                                "org.freedesktop.NetworkManager.Device.Wireless",
                            )?;

                            let options: HashMap<String, zbus::zvariant::OwnedValue> = HashMap::new();
                            if wireless_proxy.call::<_, _, ()>("RequestScan", &(options,)).is_ok() {
                                requested_scan = true;
                            }
                        }
                    }
                }
            }
        }

        if !wifi_device_found {
            return Err(crate::error::NetworkError::OperationError(
                "No wireless device available for scanning".to_string(),
            ));
        }

        if !requested_scan {
            return Err(crate::error::NetworkError::OperationError(
                "Failed to request WiFi scan on available wireless devices".to_string(),
            ));
        }

        self.list_wifi_networks()
    }

    /// Connect to a WiFi network
    pub fn connect_to_wifi(&self, config: WiFiConnectionConfig) -> Result<()> {
        // Log connection attempt
        log::debug!("connect_to_wifi called: ssid='{}' security={:?} username={:?}",
                  config.ssid, config.security_type, config.username);

        // Create connection settings
        let mut connection_settings = HashMap::new();
        let mut wifi_settings = HashMap::new();
        let mut security_settings = HashMap::new();

        // Set connection name and type
        let mut connection = HashMap::new();
        connection.insert("id".to_string(), Value::from(config.ssid.clone()));
        connection.insert("type".to_string(), Value::from("802-11-wireless"));
        connection_settings.insert("connection".to_string(), connection);

        // Set WiFi settings
        wifi_settings.insert("ssid".to_string(), Value::from(config.ssid.clone()));
        wifi_settings.insert("mode".to_string(), Value::from("infrastructure"));

        // Set security settings based on security type
        match config.security_type {
            WiFiSecurityType::None => {
                // No security settings needed
            }
            WiFiSecurityType::Wep => {
                security_settings.insert("key-mgmt".to_string(), Value::from("none"));
                if let Some(password) = config.password.clone() {
                    security_settings.insert("wep-key0".to_string(), Value::from(password));
                }
            }
            WiFiSecurityType::WpaPsk => {
                security_settings.insert("key-mgmt".to_string(), Value::from("wpa-psk"));
                if let Some(password) = config.password.clone() {
                    security_settings.insert("psk".to_string(), Value::from(password));
                }
            }
            WiFiSecurityType::WpaEap => {
                security_settings.insert("key-mgmt".to_string(), Value::from("wpa-eap"));
                if let Some(password) = config.password.clone() {
                    security_settings.insert("password".to_string(), Value::from(password));
                }
                if let Some(username) = config.username.clone() {
                    security_settings.insert("identity".to_string(), Value::from(username));
                }
            }
            WiFiSecurityType::Wpa2Psk => {
                security_settings.insert("key-mgmt".to_string(), Value::from("wpa-psk"));
                security_settings.insert("proto".to_string(), Value::from("rsn"));
                if let Some(password) = config.password.clone() {
                    security_settings.insert("psk".to_string(), Value::from(password));
                }
            }
            WiFiSecurityType::Wpa3Psk => {
                security_settings.insert("key-mgmt".to_string(), Value::from("sae"));
                if let Some(password) = config.password.clone() {
                    security_settings.insert("psk".to_string(), Value::from(password));
                }
            }
        }

        connection_settings.insert("802-11-wireless".to_string(), wifi_settings);
        connection_settings.insert("802-11-wireless-security".to_string(), security_settings);

        // Log constructed settings for debugging
        log::trace!("connection_settings: {:#?}", connection_settings);

        // Crear un proxy para NetworkManager
        let nm_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )?;

        // Llamar al método AddAndActivateConnection (trace result)
        let call_result: zbus::Result<(zbus::zvariant::OwnedObjectPath, zbus::zvariant::OwnedObjectPath)> = nm_proxy.call("AddAndActivateConnection", &(connection_settings, "/", "/"));

        match call_result {
            Ok((conn_path, active_path)) => {
                log::info!(
                    "AddAndActivateConnection succeeded for ssid='{}' conn='{}' active='{}'",
                    config.ssid,
                    conn_path.as_str(),
                    active_path.as_str()
                );
            }
            Err(e) => {
                log::error!(
                    "AddAndActivateConnection failed for ssid='{}': {:?}",
                    config.ssid,
                    e
                );
                return Err(e.into());
            }
        }

        log::debug!("connect_to_wifi finished for ssid='{}'", config.ssid);

        Ok(())
    }

    /// Toggle network state
    pub fn toggle_network_state(&self, enabled: bool) -> Result<bool> {
        let nm_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )?;

        nm_proxy.set_property("NetworkingEnabled", enabled)?;

        let current_state: bool = nm_proxy.get_property("NetworkingEnabled")?;
        Ok(current_state)
    }

    /// Get wireless enabled state
    pub fn get_wireless_enabled(&self) -> Result<bool> {
        let nm_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )?;
        Ok(nm_proxy.get_property("WirelessEnabled")?)
    }

    /// Set wireless enabled state
    pub fn set_wireless_enabled(&self, enabled: bool) -> Result<()> {
        let nm_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )?;
        nm_proxy.set_property("WirelessEnabled", enabled)?;
        Ok(())
    }

    /// Check if wireless device is available
    pub fn is_wireless_available(&self) -> Result<bool> {
         // Get all devices
        let devices_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"),
            "Devices",
        )?;

        if let Ok(zbus::zvariant::Value::Array(devices)) = devices_variant.downcast_ref() {
            for device in devices.iter() {
                if let zbus::zvariant::Value::ObjectPath(ref device_path) = device {
                     let device_props = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                            .destination("org.freedesktop.NetworkManager")?
                            .path(device_path)?
                            .build()?;
                    
                    let device_type_variant = device_props.get(
                        InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device"),
                        "DeviceType",
                    )?;
                    
                    if let Ok(zbus::zvariant::Value::U32(device_type)) = device_type_variant.downcast_ref() {
                        if device_type == 2u32 { // 2 = WiFi
                            return Ok(true);
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    /// Listen for network changes
    pub fn listen_network_changes(&self) -> Result<mpsc::Receiver<NetworkInfo>> {
        let (tx, rx) = mpsc::channel();
        let connection_clone = self.connection.clone();
        let app_handle = self.app.clone();

        // Crear un hilo para escuchar los cambios de red
        std::thread::spawn(move || {
            match zbus::blocking::Connection::system() {
                Ok(conn) => {
                    // Proxy para el objeto raíz, interfaz DBus.Properties
                    if let Ok(proxy) = zbus::blocking::Proxy::new(
                        &conn,
                        "org.freedesktop.NetworkManager",
                        "/org/freedesktop/NetworkManager",
                        "org.freedesktop.NetworkManager",
                    ) {
                        if let Ok(mut signal) = proxy.receive_signal("StateChanged") {
                            while let Some(_msg) = signal.next() {
                                let network_manager = VSKNetworkManager {
                                    connection: connection_clone.clone(),
                                    proxy: zbus::blocking::fdo::PropertiesProxy::builder(
                                        &connection_clone,
                                    )
                                    .destination("org.freedesktop.NetworkManager")
                                    .unwrap()
                                    .path("/org/freedesktop/NetworkManager")
                                    .unwrap()
                                    .build()
                                    .unwrap(),
                                    app: app_handle.clone(),
                                };

                                if let Ok(network_info) =
                                    network_manager.get_current_network_state()
                                {
                                    if tx.send(network_info).is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Error al conectar con D-Bus para escuchar cambios de red: {:?}",
                        e
                    );
                }
            }
        });

        Ok(rx)
    }

    /// Disconnect from the current WiFi network
    pub fn disconnect_from_wifi(&self) -> Result<()> {
        // Obtener el estado actual de la red para identificar la conexión activa
        let _current_state = self.get_current_network_state()?;

        // Crear un proxy para NetworkManager
        let nm_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )?;

        // Obtener las conexiones activas
        let active_connections_variant: zbus::zvariant::OwnedValue = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"),
            "ActiveConnections",
        )?;

        // Convertir el valor a un vector de ObjectPath
        let active_connections = match active_connections_variant.downcast_ref() {
            Ok(zbus::zvariant::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| match v {
                    zbus::zvariant::Value::ObjectPath(path) => {
                        Some(zbus::zvariant::OwnedObjectPath::from(path.to_owned()))
                    }
                    _ => None,
                })
                .collect::<Vec<zbus::zvariant::OwnedObjectPath>>(),
            _ => Vec::new(),
        };

        if !active_connections.is_empty() {
            nm_proxy.call::<_, _, ()>("DeactivateConnection", &(active_connections[0].as_str()))?;
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Get the list of saved WiFi networks
    pub fn get_saved_wifi_networks(&self) -> Result<Vec<NetworkInfo>> {
        // Crear un proxy para el servicio de configuración de NetworkManager
        let settings_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager/Settings",
            "org.freedesktop.NetworkManager.Settings",
        )?;

        // Obtener todas las conexiones guardadas
        let connections: Vec<zbus::zvariant::OwnedObjectPath> =
            settings_proxy.call("ListConnections", &())?;
        let mut saved_networks = Vec::new();

        // Procesar cada conexión guardada
        for conn_path in connections {
            // Crear un proxy para cada conexión
            let conn_proxy = zbus::blocking::Proxy::new(
                &self.connection,
                "org.freedesktop.NetworkManager",
                conn_path.as_str(),
                "org.freedesktop.NetworkManager.Settings.Connection",
            )?;

            // Obtener la configuración de la conexión como un HashMap
            let settings: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
                conn_proxy.call("GetSettings", &())?;

            // Verificar si es una conexión WiFi
            if let Some(connection) = settings.get("connection") {
                let connection_dict = match connection.downcast_ref::<Value>()
                    .and_then(|v| v.downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>())
                {
                    Ok(dict) => dict,
                    _ => continue,
                };

                // Verificar el tipo de conexión
                if let Some(conn_type) = connection_dict.get("type") {
                    let v_type: &zbus::zvariant::Value<'_> = conn_type;
                    let conn_type_str = match v_type.downcast_ref::<String>() {
                        Ok(s) => s,
                        _ => continue,
                    };

                    // Si es una conexión WiFi, extraer la información
                    if conn_type_str == "802-11-wireless" {
                        let mut network_info = NetworkInfo::default();
                        network_info.connection_type = "wifi".to_string();

                        // Obtener el nombre de la conexión
                        if let Some(id) = connection_dict.get("id") {
                            let v_id: &zbus::zvariant::Value<'_> = id;
                            if let Ok(name) = v_id.downcast_ref::<String>() {
                                network_info.name = name;
                            }
                        }

                        // Obtener el SSID
                        if let Some(wireless) = settings.get("802-11-wireless") {
                            let wireless_dict = match wireless.downcast_ref::<Value>()
                                .and_then(|v| v.downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>())
                            {
                                Ok(dict) => dict,
                                _ => continue,
                            };

                            if let Some(ssid) = wireless_dict.get("ssid") {
                                let v_ssid: &zbus::zvariant::Value<'_> = ssid;
                                network_info.ssid = NetworkManagerHelpers::ssid_from_value(v_ssid);
                            }
                        }

                        // Determinar el tipo de seguridad
                        if let Some(security) = settings.get("802-11-wireless-security") {
                            let security_dict = match security.downcast_ref::<Value>()
                                .and_then(|v| v.downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>())
                            {
                                Ok(dict) => dict,
                                _ => {
                                    network_info.security_type = WiFiSecurityType::None;
                                    saved_networks.push(network_info);
                                    continue;
                                },
                            };

                            if let Some(key_mgmt) = security_dict.get("key-mgmt") {
                                let v_km: &zbus::zvariant::Value<'_> = key_mgmt;
                                if let Ok(key_mgmt_str) = v_km.downcast_ref::<String>() {
                                    match key_mgmt_str.as_str() {
                                        "none" => {
                                            network_info.security_type = WiFiSecurityType::None
                                        }
                                        "wpa-psk" => {
                                            network_info.security_type = WiFiSecurityType::WpaPsk
                                        }
                                        "wpa-eap" => {
                                            network_info.security_type = WiFiSecurityType::WpaEap
                                        }
                                        _ => network_info.security_type = WiFiSecurityType::None,
                                    }
                                }
                            }
                        } else {
                            network_info.security_type = WiFiSecurityType::None;
                        }

                        // Agregar a la lista de redes guardadas
                        saved_networks.push(network_info);
                    }
                }
            }
        }

        Ok(saved_networks)
    }

    /// Delete a saved WiFi connection by SSID
    pub fn delete_wifi_connection(&self, ssid: &str) -> Result<bool> {
        // Crear un proxy para el servicio de configuración de NetworkManager
        let settings_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager/Settings",
            "org.freedesktop.NetworkManager.Settings",
        )?;

        // Obtener todas las conexiones guardadas
        let connections: Vec<zbus::zvariant::OwnedObjectPath> =
            settings_proxy.call("ListConnections", &())?;

        // Procesar cada conexión guardada
        for conn_path in connections {
            // Crear un proxy para cada conexión
            let conn_proxy = zbus::blocking::Proxy::new(
                &self.connection,
                "org.freedesktop.NetworkManager",
                conn_path.as_str(),
                "org.freedesktop.NetworkManager.Settings.Connection",
            )?;

            // Obtener la configuración de la conexión como un HashMap
            let settings: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
                conn_proxy.call("GetSettings", &())?;

            // Verificar si es una conexión WiFi
            if let Some(connection) = settings.get("connection") {
                let connection_dict = match connection.downcast_ref::<Value>()
                    .and_then(|v| v.downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>())
                {
                    Ok(dict) => dict,
                    _ => continue,
                };

                // Verificar el tipo de conexión
                if let Some(conn_type) = connection_dict.get("type") {
                    let v_type: &zbus::zvariant::Value<'_> = conn_type;
                    let conn_type_str = match v_type.downcast_ref::<String>() {
                        Ok(s) => s,
                        _ => continue,
                    };

                    // Si es una conexión WiFi, verificar el SSID
                    if conn_type_str == "802-11-wireless" {
                        if let Some(wireless) = settings.get("802-11-wireless") {
                            let wireless_dict = match wireless.downcast_ref::<Value>()
                                .and_then(|v| v.downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>())
                            {
                                Ok(dict) => dict,
                                _ => continue,
                            };

                            if let Some(ssid_value) = wireless_dict.get("ssid") {
                                let v_ssid: &zbus::zvariant::Value<'_> = ssid_value;
                                let conn_ssid_str = NetworkManagerHelpers::ssid_from_value(v_ssid);
                                // Si el SSID coincide, eliminar la conexión
                                if conn_ssid_str == ssid {
                                    conn_proxy.call::<_, _, ()>("Delete", &())?;
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }

        // No se encontró ninguna conexión con el SSID especificado
        Ok(false)
    }

    /// List saved VPN profiles from NetworkManager settings.
    pub fn list_vpn_profiles(&self) -> Result<Vec<VpnProfile>> {
        let connections = self.list_connection_paths()?;
        let mut profiles = Vec::new();

        for conn_path in connections {
            let settings = self.get_connection_settings(&conn_path)?;
            if let Some(profile) = self.vpn_profile_from_settings(&settings) {
                profiles.push(profile);
            }
        }

        profiles.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(profiles)
    }

    /// Get current VPN status from active connections.
    pub fn get_vpn_status(&self) -> Result<VpnStatus> {
        let active_connections_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"),
            "ActiveConnections",
        )?;

        let mut status = VpnStatus::default();

        if let Ok(zbus::zvariant::Value::Array(arr)) = active_connections_variant.downcast_ref() {
            for value in arr.iter() {
                let active_path = match value {
                    zbus::zvariant::Value::ObjectPath(path) => path,
                    _ => continue,
                };

                let active_props = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                    .destination("org.freedesktop.NetworkManager")?
                    .path(active_path)?
                    .build()?;

                let conn_type_variant = active_props.get(
                    InterfaceName::from_static_str_unchecked(
                        "org.freedesktop.NetworkManager.Connection.Active",
                    ),
                    "Type",
                )?;

                let conn_type = match conn_type_variant.downcast_ref() {
                    Ok(zbus::zvariant::Value::Str(v)) => v.to_string(),
                    _ => continue,
                };

                if conn_type != "vpn" {
                    continue;
                }

                let state_variant = active_props.get(
                    InterfaceName::from_static_str_unchecked(
                        "org.freedesktop.NetworkManager.Connection.Active",
                    ),
                    "State",
                )?;
                let state = match state_variant.downcast_ref() {
                    Ok(zbus::zvariant::Value::U32(v)) => v,
                    _ => 0,
                };

                let id_variant = active_props.get(
                    InterfaceName::from_static_str_unchecked(
                        "org.freedesktop.NetworkManager.Connection.Active",
                    ),
                    "Id",
                )?;
                let uuid_variant = active_props.get(
                    InterfaceName::from_static_str_unchecked(
                        "org.freedesktop.NetworkManager.Connection.Active",
                    ),
                    "Uuid",
                )?;

                status.state = Self::vpn_state_from_active_state(state);
                status.active_profile_name = match id_variant.downcast_ref() {
                    Ok(zbus::zvariant::Value::Str(v)) => Some(v.to_string()),
                    _ => None,
                };
                status.active_profile_id = status.active_profile_name.clone();
                status.active_profile_uuid = match uuid_variant.downcast_ref() {
                    Ok(zbus::zvariant::Value::Str(v)) => Some(v.to_string()),
                    _ => None,
                };

                let ip4_config_variant = active_props.get(
                    InterfaceName::from_static_str_unchecked(
                        "org.freedesktop.NetworkManager.Connection.Active",
                    ),
                    "Ip4Config",
                )?;

                if let Ok(zbus::zvariant::Value::ObjectPath(ip4_path)) =
                    ip4_config_variant.downcast_ref()
                {
                    if ip4_path.as_str() != "/" {
                        let ip4_props = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                            .destination("org.freedesktop.NetworkManager")?
                            .path(ip4_path)?
                            .build()?;

                        if let Ok(gateway_variant) = ip4_props.get(
                            InterfaceName::from_static_str_unchecked(
                                "org.freedesktop.NetworkManager.IP4Config",
                            ),
                            "Gateway",
                        ) {
                            status.gateway = match gateway_variant.downcast_ref() {
                                Ok(zbus::zvariant::Value::Str(v)) => Some(v.to_string()),
                                _ => None,
                            };
                        }
                    }
                }

                return Ok(status);
            }
        }

        Ok(status)
    }

    /// Connect a VPN profile by UUID.
    pub fn connect_vpn(&self, uuid: String) -> Result<()> {
        let current_status = self.get_vpn_status()?;
        if current_status.state == VpnConnectionState::Connected
            && current_status.active_profile_uuid.as_deref() == Some(uuid.as_str())
        {
            return Err(crate::error::NetworkError::VpnAlreadyConnected(uuid));
        }

        let conn_path = self.find_connection_path_by_uuid(&uuid)?;

        let nm_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )?;

        let activate_result: zbus::Result<zbus::zvariant::OwnedObjectPath> =
            nm_proxy.call("ActivateConnection", &(conn_path.as_str(), "/", "/"));

        match activate_result {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("secret") || msg.contains("authentication") {
                    Err(crate::error::NetworkError::VpnAuthFailed(e.to_string()))
                } else {
                    Err(crate::error::NetworkError::VpnActivationFailed(e.to_string()))
                }
            }
        }
    }

    /// Disconnect VPN by UUID or disconnect active VPN if UUID is not provided.
    pub fn disconnect_vpn(&self, uuid: Option<String>) -> Result<()> {
        let active_connections_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"),
            "ActiveConnections",
        )?;

        let mut target_active_connection: Option<zbus::zvariant::OwnedObjectPath> = None;

        if let Ok(zbus::zvariant::Value::Array(arr)) = active_connections_variant.downcast_ref() {
            for value in arr.iter() {
                let active_path = match value {
                    zbus::zvariant::Value::ObjectPath(path) => {
                        zbus::zvariant::OwnedObjectPath::from(path.to_owned())
                    }
                    _ => continue,
                };

                let active_props = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                    .destination("org.freedesktop.NetworkManager")?
                    .path(active_path.as_str())?
                    .build()?;

                let conn_type_variant = active_props.get(
                    InterfaceName::from_static_str_unchecked(
                        "org.freedesktop.NetworkManager.Connection.Active",
                    ),
                    "Type",
                )?;
                let conn_type = match conn_type_variant.downcast_ref() {
                    Ok(zbus::zvariant::Value::Str(v)) => v.to_string(),
                    _ => continue,
                };

                if conn_type != "vpn" {
                    continue;
                }

                if let Some(target_uuid) = uuid.as_deref() {
                    let uuid_variant = active_props.get(
                        InterfaceName::from_static_str_unchecked(
                            "org.freedesktop.NetworkManager.Connection.Active",
                        ),
                        "Uuid",
                    )?;
                    let active_uuid = match uuid_variant.downcast_ref() {
                        Ok(zbus::zvariant::Value::Str(v)) => v.to_string(),
                        _ => continue,
                    };

                    if active_uuid == target_uuid {
                        target_active_connection = Some(active_path.clone());
                        break;
                    }
                } else {
                    target_active_connection = Some(active_path.clone());
                    break;
                }
            }
        }

        let target_active_connection = match target_active_connection {
            Some(path) => path,
            None => {
                if let Some(target_uuid) = uuid {
                    return Err(crate::error::NetworkError::VpnProfileNotFound(target_uuid));
                }
                return Err(crate::error::NetworkError::VpnNotActive);
            }
        };

        let nm_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )?;

        nm_proxy.call::<_, _, ()>(
            "DeactivateConnection",
            &(target_active_connection.as_str(),),
        )?;
        Ok(())
    }

    /// Create a new VPN profile in NetworkManager settings.
    pub fn create_vpn_profile(&self, config: VpnCreateConfig) -> Result<VpnProfile> {
        if config.id.trim().is_empty() {
            return Err(crate::error::NetworkError::VpnInvalidConfig(
                "id is required".to_string(),
            ));
        }

        let uuid = Uuid::new_v4().to_string();
        let mut connection_section = HashMap::new();
        connection_section.insert("id".to_string(), Value::from(config.id.clone()));
        connection_section.insert("uuid".to_string(), Value::from(uuid.clone()));
        connection_section.insert("type".to_string(), Value::from("vpn"));
        connection_section.insert(
            "autoconnect".to_string(),
            Value::from(config.autoconnect.unwrap_or(false)),
        );

        let mut vpn_section = HashMap::new();
        vpn_section.insert(
            "service-type".to_string(),
            Value::from(Self::service_type_from_vpn_type(&config.vpn_type)),
        );

        if let Some(username) = config.username {
            vpn_section.insert("user-name".to_string(), Value::from(username));
        }
        if let Some(gateway) = config.gateway {
            vpn_section.insert("remote".to_string(), Value::from(gateway));
        }
        if let Some(ca_cert_path) = config.ca_cert_path {
            vpn_section.insert("ca".to_string(), Value::from(ca_cert_path));
        }
        if let Some(user_cert_path) = config.user_cert_path {
            vpn_section.insert("cert".to_string(), Value::from(user_cert_path));
        }
        if let Some(private_key_path) = config.private_key_path {
            vpn_section.insert("key".to_string(), Value::from(private_key_path));
        }
        if let Some(private_key_password) = config.private_key_password {
            vpn_section.insert("key-password".to_string(), Value::from(private_key_password));
        }
        if let Some(custom_settings) = config.settings {
            for (k, v) in custom_settings {
                vpn_section.insert(k, Value::from(v));
            }
        }

        let mut vpn_secrets_section = HashMap::new();
        if let Some(password) = config.password {
            vpn_secrets_section.insert("password".to_string(), Value::from(password));
        }
        if let Some(custom_secrets) = config.secrets {
            for (k, v) in custom_secrets {
                vpn_secrets_section.insert(k, Value::from(v));
            }
        }

        let mut settings: HashMap<String, HashMap<String, Value>> = HashMap::new();
        settings.insert("connection".to_string(), connection_section);
        settings.insert("vpn".to_string(), vpn_section);
        if !vpn_secrets_section.is_empty() {
            settings.insert("vpn-secrets".to_string(), vpn_secrets_section);
        }

        let settings_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager/Settings",
            "org.freedesktop.NetworkManager.Settings",
        )?;

        let _created_path: zbus::zvariant::OwnedObjectPath =
            settings_proxy.call("AddConnection", &(settings,))?;

        Ok(VpnProfile {
            id: config.id,
            uuid,
            vpn_type: config.vpn_type,
            interface_name: None,
            autoconnect: config.autoconnect.unwrap_or(false),
            editable: true,
            last_error: None,
        })
    }

    /// Update an existing VPN profile by UUID.
    pub fn update_vpn_profile(&self, config: VpnUpdateConfig) -> Result<VpnProfile> {
        let conn_path = self.find_connection_path_by_uuid(&config.uuid)?;
        let existing_settings = self.get_connection_settings(&conn_path)?;

        let existing_profile = self
            .vpn_profile_from_settings(&existing_settings)
            .ok_or_else(|| crate::error::NetworkError::VpnProfileNotFound(config.uuid.clone()))?;

        let existing_vpn_settings = Self::string_map_from_section(&existing_settings, "vpn");
        let existing_vpn_secrets = Self::string_map_from_section(&existing_settings, "vpn-secrets");

        // Start from the full current settings map to preserve unrelated sections
        // (IPv4/IPv6, routes, DNS, permissions, proxy, etc.).
        let mut settings: HashMap<String, HashMap<String, Value>> = HashMap::new();
        for (section_name, dict) in &existing_settings {
            let mut section_map: HashMap<String, Value> = HashMap::new();
            for (k, v) in dict {
                if let Ok(val) = v.downcast_ref::<Value>() {
                    section_map.insert(k.clone(), val);
                }
            }
            settings.insert(section_name.clone(), section_map);
        }

        let connection_section = settings
            .entry("connection".to_string())
            .or_insert_with(HashMap::new);
        connection_section.insert(
            "id".to_string(),
            Value::from(config.id.clone().unwrap_or(existing_profile.id.clone())),
        );
        connection_section.insert("uuid".to_string(), Value::from(config.uuid.clone()));
        connection_section.insert("type".to_string(), Value::from("vpn"));
        connection_section.insert(
            "autoconnect".to_string(),
            Value::from(config.autoconnect.unwrap_or(existing_profile.autoconnect)),
        );

        let service_type = existing_vpn_settings
            .get("service-type")
            .cloned()
            .unwrap_or_else(|| {
                Self::service_type_from_vpn_type(&existing_profile.vpn_type).to_string()
            });

        let vpn_section = settings
            .entry("vpn".to_string())
            .or_insert_with(HashMap::new);
        vpn_section.insert("service-type".to_string(), Value::from(service_type));

        let mut merged_settings = existing_vpn_settings;
        if let Some(username) = config.username {
            merged_settings.insert("user-name".to_string(), username);
        }
        if let Some(gateway) = config.gateway {
            merged_settings.insert("remote".to_string(), gateway);
        }
        if let Some(ca_cert_path) = config.ca_cert_path {
            merged_settings.insert("ca".to_string(), ca_cert_path);
        }
        if let Some(user_cert_path) = config.user_cert_path {
            merged_settings.insert("cert".to_string(), user_cert_path);
        }
        if let Some(private_key_path) = config.private_key_path {
            merged_settings.insert("key".to_string(), private_key_path);
        }
        if let Some(private_key_password) = config.private_key_password {
            merged_settings.insert("key-password".to_string(), private_key_password);
        }
        if let Some(custom_settings) = config.settings {
            for (k, v) in custom_settings {
                merged_settings.insert(k, v);
            }
        }

        for (k, v) in merged_settings {
            vpn_section.insert(k, Value::from(v));
        }

        let mut merged_secrets = existing_vpn_secrets;
        if let Some(password) = config.password {
            merged_secrets.insert("password".to_string(), password);
        }
        if let Some(custom_secrets) = config.secrets {
            for (k, v) in custom_secrets {
                merged_secrets.insert(k, v);
            }
        }

        if merged_secrets.is_empty() {
            settings.remove("vpn-secrets");
        } else {
            let vpn_secrets_section = settings
                .entry("vpn-secrets".to_string())
                .or_insert_with(HashMap::new);
            vpn_secrets_section.clear();
            for (k, v) in merged_secrets {
                vpn_secrets_section.insert(k, Value::from(v));
            }
        }

        let conn_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            conn_path.as_str(),
            "org.freedesktop.NetworkManager.Settings.Connection",
        )?;
        conn_proxy.call::<_, _, ()>("Update", &(settings,))?;

        let updated_settings = self.get_connection_settings(&conn_path)?;
        self.vpn_profile_from_settings(&updated_settings)
            .ok_or_else(|| crate::error::NetworkError::VpnProfileNotFound(config.uuid))
    }

    /// Delete a VPN profile by UUID.
    pub fn delete_vpn_profile(&self, uuid: String) -> Result<()> {
        let conn_path = self.find_connection_path_by_uuid(&uuid)?;

        let conn_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            conn_path.as_str(),
            "org.freedesktop.NetworkManager.Settings.Connection",
        )?;

        conn_proxy.call::<_, _, ()>("Delete", &())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vpn_state_deactivated_maps_to_disconnected() {
        assert_eq!(
            VSKNetworkManager::<tauri::Wry>::vpn_state_from_active_state(4),
            VpnConnectionState::Disconnected
        );
    }
}

/// Initialize the network manager plugin
pub async fn init(
    app: &AppHandle<tauri::Wry>,
    _api: PluginApi<tauri::Wry, ()>,
) -> Result<VSKNetworkManager<'static, tauri::Wry>> {
    Ok(VSKNetworkManager::new(app.clone()).await?)
}
