use std::collections::HashMap;
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
                    let device_props = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                        .destination("org.freedesktop.NetworkManager")?
                        .path(device_path)?
                        .build()?;

                    // Check if this is a wireless device
                    let device_type_variant = device_props.get(
                        InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device"),
                        "DeviceType",
                    )?;

                    // DeviceType 2 is WiFi
                    if let Some(zbus::zvariant::Value::U32(device_type)) = device_type_variant.downcast_ref() {
                        if device_type == &2u32 {
                            // This is a WiFi device, get its access points
                            let wireless_props = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                .destination("org.freedesktop.NetworkManager")?
                                .path(device_path)?
                                .build()?;

                            let access_points_variant = wireless_props.get(
                                InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device.Wireless"),
                                "AccessPoints",
                            )?;

                            if let Some(zbus::zvariant::Value::Array(aps)) = access_points_variant.downcast_ref() {
                                // Iterate over access points
                                let ap_values = aps.get();
                                for ap in ap_values {
                                    if let zbus::zvariant::Value::ObjectPath(ref ap_path) = ap {
                                        let ap_props = zbus::blocking::fdo::PropertiesProxy::builder(&self.connection)
                                            .destination("org.freedesktop.NetworkManager")?
                                            .path(ap_path)?
                                            .build()?;

                                        // Get SSID
                                        let ssid_variant = ap_props.get(
                                            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"),
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
                                            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"),
                                            "Strength",
                                        )?;

                                        let strength = match strength_variant.downcast_ref() {
                                            Some(zbus::zvariant::Value::U8(s)) => *s,
                                            _ => 0,
                                        };

                                        // Get security flags
                                        let _flags_variant = ap_props.get(
                                            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"),
                                            "Flags",
                                        )?;

                                        let wpa_flags_variant = ap_props.get(
                                            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"),
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
                                            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device"),
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
                                            ip_address: if is_connected { current_network.ip_address.clone() } else { "0.0.0.0".to_string() },
                                            mac_address,
                                            signal_strength: strength,
                                            security_type,
                                            is_connected,
                                        };

                                        // Add to list if not already present
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

    /// Connect to a WiFi network
    pub fn connect_to_wifi(&self, config: WiFiConnectionConfig) -> Result<()> {
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
            "org.freedesktop.NetworkManager"
        )?;
        
        // Llamar al método AddAndActivateConnection
        let result: (zbus::zvariant::OwnedObjectPath, zbus::zvariant::OwnedObjectPath) = nm_proxy.call(
            "AddAndActivateConnection",
            &(connection_settings, "/", "/")
        )?;
        
        // Si llegamos aquí, la conexión fue exitosa
        println!("Conexión creada: {:?}, activada: {:?}", result.0, result.1);
        Ok(())
    }

    /// Toggle network state
    pub fn toggle_network_state(&self, enabled: bool) -> Result<bool> {
        // Crear un proxy para NetworkManager
        let nm_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        )?;
        
        // Establecer el estado de la red (habilitado/deshabilitado)
        nm_proxy.set_property("WirelessEnabled", enabled)?;
        
        // Verificar que el estado se haya actualizado correctamente
        let current_state: bool = nm_proxy.get_property("WirelessEnabled")?;
        
        // Devolver el estado actual
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
                        "org.freedesktop.NetworkManager"
                    ) {
                        // Configurar un manejador para la señal PropertiesChanged
                        if let Ok(mut signal) = proxy.receive_signal("PropertiesChanged") {
                            // Bucle para procesar señales
                            while let Some(_msg) = signal.next() {
                                // Intentar obtener el estado actual de la red
                                let network_manager = VSKNetworkManager {
                                    connection: connection_clone.clone(),
                                    proxy: zbus::blocking::fdo::PropertiesProxy::builder(&connection_clone)
                                        .destination("org.freedesktop.NetworkManager")
                                        .unwrap()
                                        .path("/org/freedesktop/NetworkManager")
                                        .unwrap()
                                        .build()
                                        .unwrap(),
                                    app: app_handle.clone(),
                                };
                                
                                if let Ok(network_info) = network_manager.get_current_network_state() {
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
                    eprintln!("Error al conectar con D-Bus para escuchar cambios de red: {:?}", e);
                }
            }
        });
        
        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wifi_icon() {
        assert_eq!(VSKNetworkManager::<tauri::Wry>::get_wifi_icon(0), "wifi-signal-weak");
        assert_eq!(VSKNetworkManager::<tauri::Wry>::get_wifi_icon(30), "wifi-signal-low");
        assert_eq!(VSKNetworkManager::<tauri::Wry>::get_wifi_icon(50), "wifi-signal-medium");
        assert_eq!(VSKNetworkManager::<tauri::Wry>::get_wifi_icon(70), "wifi-signal-good");
        assert_eq!(VSKNetworkManager::<tauri::Wry>::get_wifi_icon(90), "wifi-signal-excellent");
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
