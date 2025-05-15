use std::collections::HashMap;
use std::sync::mpsc;
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use zbus::names::InterfaceName;
use zbus::zvariant::Value;

use crate::error::{NetworkError, Result};
use crate::models::*;

impl<R: Runtime> VSKNetworkManager<'static, R> {
    /// Get WiFi icon based on signal strength
    fn get_wifi_icon(strength: u8) -> String {
        match strength {
            0..=20 => "wifi-signal-weak".to_string(),
            21..=40 => "wifi-signal-low".to_string(),
            41..=60 => "wifi-signal-medium".to_string(),
            61..=80 => "wifi-signal-good".to_string(),
            81..=100 => "wifi-signal-excellent".to_string(),
            _ => "wifi-signal-none".to_string(),
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
        // Create a NetworkManager proxy
        let _network_manager_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )?;

        // Get active connections
        let active_connections_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"),
            "ActiveConnections",
        )?;

        // If no active connections, return default
        match active_connections_variant.downcast_ref() {
            Some(Value::Array(arr)) if !arr.is_empty() => {
                // Get the first active connection path
                match arr[0] {
                    zbus::zvariant::Value::ObjectPath(ref path) => {
                        // Create a proxy for the active connection
                        let _connection_proxy = zbus::blocking::Proxy::new(
                            &self.connection,
                            "org.freedesktop.NetworkManager",
                            path,
                            "org.freedesktop.NetworkManager.Connection.Active",
                        )?;

                        // Get devices for this connection
                        // Crear un proxy de propiedades para obtener las propiedades
                        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
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
                            Some(Value::Array(device_arr)) if !device_arr.is_empty() => {
                                match device_arr[0] {
                                    zbus::zvariant::Value::ObjectPath(ref dev_path) => {
                                        dev_path.clone()
                                    }
                                    _ => return Ok(NetworkInfo::default()),
                                }
                            }
                            _ => return Ok(NetworkInfo::default()),
                        };

                        // Create a device proxy
                        let _device_proxy = zbus::blocking::Proxy::new(
                            &self.connection,
                            "org.freedesktop.NetworkManager",
                            &device_path,
                            "org.freedesktop.NetworkManager.Device",
                        )?;

                        // Retrieve connection details
                        // Crear un proxy de propiedades para el dispositivo
                        let device_properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                            .destination("org.freedesktop.NetworkManager")?
                            .path(&device_path)?
                            .build()?;
                        
                        let connection_type = device_properties_proxy.get(
                            InterfaceName::from_static_str_unchecked(
                                "org.freedesktop.NetworkManager.Device",
                            ),
                            "DeviceType",
                        )?;

                        // Determine connection type
                        let connection_type_str = match connection_type.downcast_ref() {
                            Some(zbus::zvariant::Value::U32(device_type)) => match device_type {
                                2 => "Ethernet".to_string(),
                                3 => "WiFi".to_string(),
                                _ => "Unknown".to_string(),
                            },
                            _ => "Unknown".to_string(),
                        };

                        // Default network info
                        let mut network_info = NetworkInfo {
                            name: "Unknown".to_string(),
                            ssid: "Unknown".to_string(),
                            connection_type: connection_type_str,
                            icon: "network-offline".to_string(),
                            ip_address: "0.0.0.0".to_string(),
                            mac_address: "00:00:00:00:00:00".to_string(),
                            signal_strength: 0,
                            security_type: WiFiSecurityType::None,
                            is_connected: false,
                        };

                        // For WiFi networks, get additional details
                        if let Ok(_wireless_proxy) = zbus::blocking::Proxy::new(
                            &self.connection,
                            "org.freedesktop.NetworkManager",
                            &device_path,
                            "org.freedesktop.NetworkManager.Device.Wireless",
                        ) {
                            // Get active access point
                            // Crear un proxy de propiedades para el dispositivo inalámbrico
                            let wireless_properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                .destination("org.freedesktop.NetworkManager")?
                                .path(&device_path)?
                                .build()?;
                            
                            let active_ap_path = wireless_properties_proxy.get(
                                InterfaceName::from_static_str_unchecked(
                                    "org.freedesktop.NetworkManager.Device.Wireless",
                                ),
                                "ActiveAccessPoint",
                            )?;

                            if let Some(zbus::zvariant::Value::ObjectPath(ap_path)) =
                                active_ap_path.downcast_ref()
                            {
                                let _ap_proxy = zbus::blocking::Proxy::new(
                                    &self.connection,
                                    "org.freedesktop.NetworkManager",
                                    ap_path,
                                    "org.freedesktop.NetworkManager.AccessPoint",
                                )?;

                                // Get SSID
                                // Crear un proxy de propiedades para el punto de acceso
                                let ap_properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                    .destination("org.freedesktop.NetworkManager")?
                                    .path(ap_path)?
                                    .build()?;
                                
                                let ssid_variant = ap_properties_proxy.get(
                                    InterfaceName::from_static_str_unchecked(
                                        "org.freedesktop.NetworkManager.AccessPoint",
                                    ),
                                    "Ssid",
                                )?;

                                network_info.ssid = match ssid_variant.downcast_ref() {
                                    Some(zbus::zvariant::Value::Array(ssid_bytes)) => {
                                        // Convertir el array de bytes a una cadena UTF-8
                                        let bytes: Vec<u8> = ssid_bytes
                                            .iter()
                                            .filter_map(|v| {
                                                if let zbus::zvariant::Value::U8(b) = v {
                                                    Some(*b)
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();
                                        
                                        String::from_utf8_lossy(&bytes).to_string()
                                    }
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
                                    Some(zbus::zvariant::Value::U8(s)) => *s,
                                    _ => 0,
                                };

                                // Update icon based on signal strength
                                network_info.icon =
                                    Self::get_wifi_icon(network_info.signal_strength);

                                // Determine security type
                                let wpa_flags_variant = ap_properties_proxy.get(
                                    InterfaceName::from_static_str_unchecked(
                                        "org.freedesktop.NetworkManager.AccessPoint",
                                    ),
                                    "WpaFlags",
                                )?;

                                network_info.security_type = match wpa_flags_variant.downcast_ref()
                                {
                                    Some(zbus::zvariant::Value::U32(flags)) => {
                                        if *flags == 0 {
                                            WiFiSecurityType::None
                                        } else {
                                            WiFiSecurityType::WpaPsk
                                        }
                                    }
                                    _ => WiFiSecurityType::None,
                                };
                            }
                        }

                        // Get IP configuration
                        let ip4_config_path = device_properties_proxy.get(
                            InterfaceName::from_static_str_unchecked(
                                "org.freedesktop.NetworkManager.Device",
                            ),
                            "Ip4Config",
                        )?;

                        // Retrieve IP address if available
                        if let Some(zbus::zvariant::Value::ObjectPath(config_path)) =
                            ip4_config_path.downcast_ref()
                        {
                            let _ip_config_proxy = zbus::blocking::Proxy::new(
                                &self.connection,
                                "org.freedesktop.NetworkManager",
                                config_path,
                                "org.freedesktop.NetworkManager.IP4Config",
                            )?;

                            // Crear un proxy de propiedades para la configuración IP
                            let ip_config_properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                .destination("org.freedesktop.NetworkManager")?
                                .path(config_path)?
                                .build()?;
                            
                            let addresses_variant = ip_config_properties_proxy.get(
                                InterfaceName::from_static_str_unchecked(
                                    "org.freedesktop.NetworkManager.IP4Config",
                                ),
                                "Addresses",
                            )?;

                            if let Some(Value::Array(addr_arr)) = addresses_variant.downcast_ref() {
                                if !addr_arr.is_empty() {
                                    if let zbus::zvariant::Value::Structure(ref addr_tuple) =
                                        addr_arr[0]
                                    {
                                        // Acceder a los elementos de la estructura
                                        let fields = addr_tuple.fields();
                                        if !fields.is_empty() {
                                            if let Some(zbus::zvariant::Value::U32(ip_int)) =
                                                fields[0].downcast_ref()
                                            {
                                                network_info.ip_address = format!(
                                                    "{}.{}.{}.{}",
                                                    (ip_int & 0xFF),
                                                    (ip_int >> 8) & 0xFF,
                                                    (ip_int >> 16) & 0xFF,
                                                    (ip_int >> 24) & 0xFF
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Mark as connected if we have a valid IP
                        network_info.is_connected = network_info.ip_address != "0.0.0.0";

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
        // Get wireless devices
        let _devices_proxy = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
            .destination("org.freedesktop.NetworkManager")?
            .path("/org/freedesktop/NetworkManager")?;

        // Return a default list for now
        // TODO: Implement actual WiFi network scanning
        Ok(vec![NetworkInfo {
            name: "TestNetwork".to_string(),
            ssid: "TestNetwork".to_string(),
            connection_type: "wifi".to_string(),
            icon: Self::get_wifi_icon(75),
            ip_address: "192.168.1.100".to_string(),
            mac_address: "00:11:22:33:44:55".to_string(),
            signal_strength: 75,
            security_type: WiFiSecurityType::WpaPsk,
            is_connected: false,
        }])
    }

    /// Connect to a WiFi network
    pub fn connect_to_wifi(&self, config: WiFiConnectionConfig) -> Result<()> {
        let _settings_proxy = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
            .destination("org.freedesktop.NetworkManager")?
            .path("/org/freedesktop/NetworkManager/Settings")?;

        // Prepare connection settings
        let mut connection_settings = HashMap::new();
        let mut wifi_settings = HashMap::new();
        let mut security_settings = HashMap::new();

        // Basic WiFi settings
        wifi_settings.insert("ssid".to_string(), config.ssid);
        wifi_settings.insert("mode".to_string(), "infrastructure".to_string());

        // Security settings based on type
        match config.security_type {
            WiFiSecurityType::WpaPsk => {
                if let Some(password) = config.password {
                    security_settings.insert("key-mgmt".to_string(), "wpa-psk".to_string());
                    security_settings.insert("psk".to_string(), password);
                }
            }
            WiFiSecurityType::None => {
                security_settings.insert("key-mgmt".to_string(), "none".to_string());
            }
            _ => return Err(NetworkError::UnsupportedSecurityType),
        }

        // Add settings to main connection settings
        connection_settings.insert("802-11-wireless".to_string(), wifi_settings);
        connection_settings.insert("802-11-wireless-security".to_string(), security_settings);

        // TODO: Implement actual connection
        // For now, return NotImplemented
        Err(NetworkError::NotImplemented)
    }

    /// Toggle network state
    pub fn toggle_network_state(&self, _enabled: bool) -> Result<bool> {
        let _properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
            .destination("org.freedesktop.NetworkManager")?
            .path("/org/freedesktop/NetworkManager")?;

        // TODO: Implement actual network state toggling
        // For now, return NotImplemented
        Err(NetworkError::NotImplemented)
    }

    /// Listen for network changes
    pub fn listen_network_changes(&self) -> Result<mpsc::Receiver<NetworkInfo>> {
        let (_tx, rx) = mpsc::channel();

        // TODO: Implement network change monitoring
        // For now, just return the receiver
        Ok(rx)
    }
}

/// Initialize the network manager plugin
pub async fn init(
    app: &AppHandle<tauri::Wry>,
    _api: PluginApi<tauri::Wry, ()>,
) -> Result<VSKNetworkManager<'static, tauri::Wry>> {
    // Initialize the network manager
    VSKNetworkManager::new(app.clone()).await
}
