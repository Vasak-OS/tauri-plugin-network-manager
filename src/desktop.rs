use std::collections::HashMap;
use std::process::Command;
use std::sync::mpsc;
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use zbus::names::InterfaceName;
use zbus::zvariant::Value;

use crate::error::Result;
use crate::models::*;

impl<R: Runtime> VSKNetworkManager<'static, R> {
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

    fn has_internet_connectivity() -> bool {
        Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg("8.8.8.8")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub fn get_current_network_state(&self) -> Result<NetworkInfo> {
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
                            Some(zbus::zvariant::Value::U32(state)) => *state == 2, // 2 = ACTIVATED
                            _ => false,
                        };

                        // Determine connection type
                        let connection_type_str = match connection_type.downcast_ref() {
                            Some(zbus::zvariant::Value::U32(device_type)) => match device_type {
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
                            is_connected: is_connected && Self::has_internet_connectivity(),
                        };

                        let hw_address_variant = device_properties_proxy.get(
                            InterfaceName::from_static_str_unchecked(
                                "org.freedesktop.NetworkManager.Device",
                            ),
                            "HwAddress",
                        )?;

                        network_info.mac_address = match hw_address_variant.downcast_ref() {
                            Some(zbus::zvariant::Value::Str(s)) => s.to_string(),
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
                                let ap_properties_proxy =
                                    zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
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
                        if let Some(zbus::zvariant::Value::ObjectPath(config_path)) =
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

                            if let Some(Value::Array(addr_arr)) = addresses_variant.downcast_ref() {
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
                    _ => {
                        println!("No valid device path found");
                        Ok(NetworkInfo::default())
                    }
                }
            }
            _ => {
                println!("No active connections found");
                Ok(NetworkInfo::default())
            }
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

        if let Some(zbus::zvariant::Value::Array(devices)) = devices_variant.downcast_ref() {
            // Iterate over devices in the array
            let device_values = devices.get();
            for device in device_values {
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
                    if let Some(zbus::zvariant::Value::U32(device_type)) =
                        device_type_variant.downcast_ref()
                    {
                        if device_type == &2u32 {
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

                            if let Some(zbus::zvariant::Value::Array(aps)) =
                                access_points_variant.downcast_ref()
                            {
                                // Iterate over access points
                                let ap_values = aps.get();
                                for ap in ap_values {
                                    if let zbus::zvariant::Value::ObjectPath(ref ap_path) = ap {
                                        let ap_props =
                                            zbus::blocking::fdo::PropertiesProxy::builder(
                                                &self.connection,
                                            )
                                            .destination("org.freedesktop.NetworkManager")?
                                            .path(ap_path)?
                                            .build()?;

                                        // Get SSID
                                        let ssid_variant = ap_props.get(
                                            InterfaceName::from_static_str_unchecked(
                                                "org.freedesktop.NetworkManager.AccessPoint",
                                            ),
                                            "Ssid",
                                        )?;

                                        let ssid = match ssid_variant.downcast_ref() {
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

                                        // Get signal strength
                                        let strength_variant = ap_props.get(
                                            InterfaceName::from_static_str_unchecked(
                                                "org.freedesktop.NetworkManager.AccessPoint",
                                            ),
                                            "Strength",
                                        )?;

                                        let strength = match strength_variant.downcast_ref() {
                                            Some(zbus::zvariant::Value::U8(s)) => *s,
                                            _ => 0,
                                        };

                                        // Get security flags
                                        let _flags_variant = ap_props.get(
                                            InterfaceName::from_static_str_unchecked(
                                                "org.freedesktop.NetworkManager.AccessPoint",
                                            ),
                                            "Flags",
                                        )?;

                                        let wpa_flags_variant = ap_props.get(
                                            InterfaceName::from_static_str_unchecked(
                                                "org.freedesktop.NetworkManager.AccessPoint",
                                            ),
                                            "WpaFlags",
                                        )?;

                                        let security_type = match wpa_flags_variant.downcast_ref() {
                                            Some(zbus::zvariant::Value::U32(flags)) => {
                                                if *flags == 0 {
                                                    WiFiSecurityType::None
                                                } else {
                                                    WiFiSecurityType::WpaPsk
                                                }
                                            }
                                            _ => WiFiSecurityType::None,
                                        };

                                        // Get hardware address (MAC)
                                        let hw_address_variant = device_props.get(
                                            InterfaceName::from_static_str_unchecked(
                                                "org.freedesktop.NetworkManager.Device",
                                            ),
                                            "HwAddress",
                                        )?;

                                        let mac_address = match hw_address_variant.downcast_ref() {
                                            Some(zbus::zvariant::Value::Str(s)) => s.to_string(),
                                            _ => "00:00:00:00:00:00".to_string(),
                                        };

                                        // Check if this is the currently connected network
                                        let is_connected = current_network.ssid == ssid;

                                        // Create network info
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
                                            mac_address,
                                            signal_strength: strength,
                                            security_type,
                                            is_connected,
                                        };

                                        // Add to list if not already present
                                        if !networks
                                            .iter()
                                            .any(|n: &NetworkInfo| n.ssid == network_info.ssid)
                                        {
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

    /// Connect to a WiFi network
    pub async fn connect_to_wifi(&self, config: WiFiConnectionConfig) -> Result<()> {
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
                if let Some(password) = config.password {
                    security_settings.insert("wep-key0".to_string(), Value::from(password));
                }
            }
            WiFiSecurityType::WpaPsk => {
                security_settings.insert("key-mgmt".to_string(), Value::from("wpa-psk"));
                if let Some(password) = config.password {
                    security_settings.insert("psk".to_string(), Value::from(password));
                }
            }
            WiFiSecurityType::WpaEap => {
                security_settings.insert("key-mgmt".to_string(), Value::from("wpa-eap"));
                if let Some(password) = config.password {
                    security_settings.insert("password".to_string(), Value::from(password));
                }
                if let Some(username) = config.username {
                    security_settings.insert("identity".to_string(), Value::from(username));
                }
            }
            WiFiSecurityType::Wpa2Psk => {
                security_settings.insert("key-mgmt".to_string(), Value::from("wpa-psk"));
                security_settings.insert("proto".to_string(), Value::from("rsn"));
                if let Some(password) = config.password {
                    security_settings.insert("psk".to_string(), Value::from(password));
                }
            }
            WiFiSecurityType::Wpa3Psk => {
                security_settings.insert("key-mgmt".to_string(), Value::from("sae"));
                if let Some(password) = config.password {
                    security_settings.insert("psk".to_string(), Value::from(password));
                }
            }
        }

        connection_settings.insert("802-11-wireless".to_string(), wifi_settings);
        connection_settings.insert("802-11-wireless-security".to_string(), security_settings);

        // Crear un proxy para NetworkManager
        let nm_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )?;

        // Llamar al método AddAndActivateConnection
        let result: (
            zbus::zvariant::OwnedObjectPath,
            zbus::zvariant::OwnedObjectPath,
        ) = nm_proxy.call("AddAndActivateConnection", &(connection_settings, "/", "/"))?;

        // Si llegamos aquí, la conexión fue exitosa
        println!("Conexión creada: {:?}, activada: {:?}", result.0, result.1);
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

        let state = if enabled { "on" } else { "off" };
        let output = Command::new("nmcli")
            .arg("networking")
            .arg(state)
            .output()?;

        if !output.status.success() {
            println!(
                "nmcli networking {} failed: {}",
                state,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Verifica el estado general
        let current_state: bool = nm_proxy.get_property("NetworkingEnabled")?;
        Ok(current_state)
    }

    /// Listen for network changes
    pub fn listen_network_changes(&self) -> Result<mpsc::Receiver<NetworkInfo>> {
        let (tx, rx) = mpsc::channel();
        let connection_clone = self.connection.clone();
        let app_handle = self.app.clone();

        // Crear un hilo para escuchar los cambios de red
        std::thread::spawn(move || {
            // Intentar crear una conexión para escuchar eventos
            match zbus::blocking::Connection::system() {
                Ok(conn) => {
                    // Crear un proxy para las señales de NetworkManager
                    if let Ok(proxy) = zbus::blocking::Proxy::new(
                        &conn,
                        "org.freedesktop.NetworkManager",
                        "/org/freedesktop/NetworkManager",
                        "org.freedesktop.NetworkManager",
                    ) {
                        // Configurar un manejador para la señal PropertiesChanged
                        if let Ok(mut signal) = proxy.receive_signal("PropertiesChanged") {
                            // Bucle para procesar señales
                            while let Some(_msg) = signal.next() {
                                // Intentar obtener el estado actual de la red
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
                                    // Enviar la información de la red actualizada
                                    if tx.send(network_info).is_err() {
                                        // El receptor fue cerrado, salir del bucle
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
    pub async fn disconnect_from_wifi(&self) -> Result<()> {
        // Obtener el estado actual de la red para identificar la conexión activa
        let current_state = self.get_current_network_state()?;

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
            Some(zbus::zvariant::Value::Array(arr)) => arr
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

        // Si hay conexiones activas, desconectar la primera (normalmente solo hay una para WiFi)
        if !active_connections.is_empty() {
            // Llamar al método DeactivateConnection para desconectar
            nm_proxy.call::<_, _, ()>("DeactivateConnection", &(active_connections[0].as_str()))?;
            println!("Desconectado de la red: {}", current_state.ssid);
            Ok(())
        } else {
            // No hay conexiones activas
            println!("No hay conexiones activas para desconectar");
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
                let connection_value = connection.to_owned();
                let connection_dict =
                    match <zbus::zvariant::Value<'_> as Clone>::clone(&connection_value)
                        .downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>(
                    ) {
                        Some(dict) => dict,
                        None => continue,
                    };

                // Verificar el tipo de conexión
                if let Some(conn_type) = connection_dict.get("type") {
                    let conn_type_value = conn_type.to_owned();
                    let conn_type_str =
                        match <zbus::zvariant::Value<'_> as Clone>::clone(&conn_type_value)
                            .downcast::<String>()
                        {
                            Some(s) => s,
                            None => continue,
                        };

                    // Si es una conexión WiFi, extraer la información
                    if conn_type_str == "802-11-wireless" {
                        let mut network_info = NetworkInfo::default();
                        network_info.connection_type = "wifi".to_string();

                        // Obtener el nombre de la conexión
                        if let Some(id) = connection_dict.get("id") {
                            let id_value = id.to_owned();
                            if let Some(name) =
                                <zbus::zvariant::Value<'_> as Clone>::clone(&id_value)
                                    .downcast::<String>()
                            {
                                network_info.name = name;
                            }
                        }

                        // Obtener el SSID
                        if let Some(wireless) = settings.get("802-11-wireless") {
                            let wireless_value = wireless.to_owned();
                            let wireless_dict = match <zbus::zvariant::Value<'_> as Clone>::clone(&wireless_value).downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>() {
                                Some(dict) => dict,
                                None => continue,
                            };

                            if let Some(ssid) = wireless_dict.get("ssid") {
                                let ssid_value = ssid.to_owned();
                                if let Some(ssid_bytes) =
                                    <zbus::zvariant::Value<'_> as Clone>::clone(&ssid_value)
                                        .downcast::<Vec<u8>>()
                                {
                                    if let Ok(ssid_str) = String::from_utf8(ssid_bytes) {
                                        network_info.ssid = ssid_str;
                                    }
                                }
                            }
                        }

                        // Determinar el tipo de seguridad
                        if let Some(security) = settings.get("802-11-wireless-security") {
                            let security_value = security.to_owned();
                            let security_dict = match <zbus::zvariant::Value<'_> as Clone>::clone(&security_value).downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>() {
                                Some(dict) => dict,
                                None => {
                                    network_info.security_type = WiFiSecurityType::None;
                                    saved_networks.push(network_info);
                                    continue;
                                },
                            };

                            if let Some(key_mgmt) = security_dict.get("key-mgmt") {
                                let key_mgmt_value = key_mgmt.to_owned();
                                if let Some(key_mgmt_str) =
                                    <zbus::zvariant::Value<'_> as Clone>::clone(&key_mgmt_value)
                                        .downcast::<String>()
                                {
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
                let connection_value = connection.to_owned();
                let connection_dict =
                    match <zbus::zvariant::Value<'_> as Clone>::clone(&connection_value)
                        .downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>(
                    ) {
                        Some(dict) => dict,
                        None => continue,
                    };

                // Verificar el tipo de conexión
                if let Some(conn_type) = connection_dict.get("type") {
                    let conn_type_value = conn_type.to_owned();
                    let conn_type_str =
                        match <zbus::zvariant::Value<'_> as Clone>::clone(&conn_type_value)
                            .downcast::<String>()
                        {
                            Some(s) => s,
                            None => continue,
                        };

                    // Si es una conexión WiFi, verificar el SSID
                    if conn_type_str == "802-11-wireless" {
                        if let Some(wireless) = settings.get("802-11-wireless") {
                            let wireless_value = wireless.to_owned();
                            let wireless_dict = match <zbus::zvariant::Value<'_> as Clone>::clone(&wireless_value).downcast::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>() {
                                Some(dict) => dict,
                                None => continue,
                            };

                            if let Some(ssid_value) = wireless_dict.get("ssid") {
                                let ssid_owned = ssid_value.to_owned();
                                if let Some(ssid_bytes) =
                                    <zbus::zvariant::Value<'_> as Clone>::clone(&ssid_owned)
                                        .downcast::<Vec<u8>>()
                                {
                                    if let Ok(conn_ssid_str) = String::from_utf8(ssid_bytes) {
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
            }
        }

        // No se encontró ninguna conexión con el SSID especificado
        Ok(false)
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
