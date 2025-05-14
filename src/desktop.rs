use crate::error::Result;
use serde::{Deserialize, Serialize};
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use std::collections::HashMap;
use std::sync::mpsc;
use zbus::{names::InterfaceName, zvariant::{OwnedValue, Value}};
// Removed unused import
use crate::error::NetworkError;

trait WirelessDeviceProxy {
    fn get_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
}

trait DeviceProxy {
    fn connection(&self) -> &zbus::blocking::Connection;
    fn destination(&self) -> &str;
    fn path(&self) -> &str;
    fn device_type(&self) -> zbus::Result<u32>;
    fn get_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.Device.Wireless")?
            .build()?;
        
        let aps_variant: OwnedValue = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device.Wireless"), "AccessPoints")?.try_into()?;
        
        match aps_variant.downcast_ref() {
            Some(Value::Array(arr)) => Ok(arr.into_iter()
                .filter_map(|v| match v {
                    zbus::zvariant::Value::ObjectPath(path) => Some(zbus::zvariant::OwnedObjectPath::from(path.to_owned())),
                    _ => None,
                })
                .collect()),
            _ => Err(zbus::Error::Failure("Failed to parse access points".into())),
        }
    }
}

impl<'a> DeviceProxy for zbus::blocking::Proxy<'a> {
    fn connection(&self) -> &zbus::blocking::Connection {
        // Convert blocking connection to zbus::Connection
        self.connection()
    }
    
    fn destination(&self) -> &str {
        zbus::blocking::Proxy::destination(self)
    }
    
    fn path(&self) -> &str {
        zbus::blocking::Proxy::path(self)
    }
    fn device_type(&self) -> zbus::Result<u32> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.Device")?
            .build()?;
        
        let device_type_variant = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device"), "DeviceType")?;
        
        match device_type_variant.downcast_ref() {
            Some(zbus::zvariant::Value::U32(device_type)) => Ok(*device_type),
            _ => Err(zbus::Error::from(std::io::Error::new(std::io::ErrorKind::Other, "Failed to parse device type"))),
        }
    }
}

impl WirelessDeviceProxy for zbus::blocking::Proxy<'_> {
    fn get_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.Device.Wireless")?
            .build()?;
        
        let aps_variant: OwnedValue = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device.Wireless"), "AccessPoints")?.try_into()?;
        
        match aps_variant.downcast_ref() {
            Some(Value::Array(arr)) => Ok(arr.into_iter()
                .filter_map(|v| match v {
                    zbus::zvariant::Value::ObjectPath(path) => Some(zbus::zvariant::OwnedObjectPath::from(path.to_owned())),
                    _ => None,
                })
                .collect()),
            _ => Err(zbus::Error::Failure("Failed to parse access points".into())),
        }
    }
}

trait AccessPointProxy {
    fn ssid(&self) -> zbus::Result<Vec<u8>>;
    fn strength(&self) -> zbus::Result<u8>;
    fn security_type(&self) -> zbus::Result<WiFiSecurityType>;
}

impl AccessPointProxy for zbus::blocking::Proxy<'_> {
    fn ssid(&self) -> zbus::Result<Vec<u8>> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.AccessPoint")?
            .build()?;
        
        let ssid_variant = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"), &InterfaceName::from_static_str_unchecked("Ssid"))?;
        
        match ssid_variant.downcast_ref() {
            Some(zbus::zvariant::Value::Array(v)) => {
                let bytes: Vec<u8> = v.iter()
                    .filter_map(|val| match val {
                        zbus::zvariant::Value::U8(byte) => Some(*byte),
                        _ => None,
                    })
                    .collect();
                Ok(bytes)
            },
            _ => Err(zbus::Error::Failure("Failed to parse SSID".into())),
        }
    }
    
    fn strength(&self) -> zbus::Result<u8> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.AccessPoint")?
            .build()?;
        
        let strength_variant = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"), &InterfaceName::from_static_str_unchecked("Strength"))?;
        
        match strength_variant.downcast_ref() {
            Some(zbus::zvariant::Value::U8(v)) => Ok(*v),
            _ => Err(zbus::Error::Failure("Failed to parse strength".into())),
        }
    }
    
    fn security_type(&self) -> zbus::Result<WiFiSecurityType> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.AccessPoint")?
            .build()?;
        
        let wpa_flags_variant = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"), &InterfaceName::from_static_str_unchecked("WpaFlags"))?;
        
        let wpa_flags: u32 = match wpa_flags_variant.downcast_ref() {
            Some(zbus::zvariant::Value::U32(v)) => Ok(*v),
            _ => Err(zbus::Error::Failure("Failed to parse WPA flags".into())),
        }?;
        
        Ok(match wpa_flags {
            0 => WiFiSecurityType::None,
            _ => WiFiSecurityType::WpaPsk,
        })
    }
}

trait ConnectionProxy {
    fn add_connection(&self, settings: &HashMap<String, HashMap<String, String>>) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn call_add_connection(&self, settings: &HashMap<String, HashMap<String, String>>) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

impl ConnectionProxy for zbus::blocking::Proxy<'_> {
    fn add_connection(&self, _settings: &HashMap<String, HashMap<String, String>>) -> zbus::Result<zbus::zvariant::OwnedObjectPath> {
        // TODO: Implement actual connection addition logic
        Err(zbus::Error::Failure("Connection addition not implemented".into()))
    }
    
    fn call_add_connection(&self, _settings: &HashMap<String, HashMap<String, String>>) -> zbus::Result<zbus::zvariant::OwnedObjectPath> {
        // TODO: Implement actual connection addition logic
        Err(zbus::Error::Failure("Connection addition not implemented".into()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInfo {
    pub name: String,
    pub ssid: String,
    pub connection_type: String,
    pub icon: String,
    pub ip_address: String,
    pub mac_address: String,
    pub signal_strength: u8,
    pub security_type: WiFiSecurityType,
    pub is_connected: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum WiFiSecurityType {
    #[default]
    None,
    Wep,
    WpaPsk,
    WpaEap,
    Wpa2Psk,
    Wpa3Psk,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WiFiConnectionConfig {
    pub ssid: String,
    pub password: Option<String>,
    pub security_type: WiFiSecurityType,
    pub username: Option<String>,
}

// Removed duplicate init function

#[derive(Clone)]
pub struct VSKNetworkManager<'a, R: Runtime> {
    pub connection: zbus::blocking::Connection,
    pub proxy: zbus::blocking::fdo::PropertiesProxy<'a>,
    pub app: AppHandle<R>,
}

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
        // TODO: Implement proper network state retrieval
        let _network_manager_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        )?;

        // Get active connections
        let _active_connections_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"), 
            "ActiveConnections"
        ).map_err(|e| NetworkError::from(e))?;

        // Parse active connections
        let _active_connections_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        )?;

        // Get active connections
        let active_connections_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"), 
            "ActiveConnections"
        )?;

        // If no active connections, return default
        let active_connections = match active_connections_variant.downcast_ref() {
            Some(Value::Array(arr)) if !arr.is_empty() => {
                // Get the first active connection path
                match arr[0] {
                    zbus::zvariant::Value::ObjectPath(ref path) => {
                        // Create a proxy for the active connection
                        let _connection_proxy = zbus::blocking::Proxy::new(
                            &self.connection,
                            "org.freedesktop.NetworkManager",
                            path,
                            "org.freedesktop.NetworkManager.Connection.Active"
                        )?;

                        // Get connection details
                        // Retrieve connection type and SSID directly from the connection proxy
                        let connection_type_variant = self.proxy.get(
                            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Connection.Active"), 
                            "Type"
                        )?;

                        let ssid_variant = self.proxy.get(
                            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Connection.Active"), 
                            "Id"
                        )?;

                        // Extract connection type and SSID
                        let connection_type = match connection_type_variant.downcast_ref() {
                            Some(Value::Str(s)) => s.to_string(),
                            _ => "Unknown".to_string(),
                        };

                        let ssid = match ssid_variant.downcast_ref() {
                            Some(Value::Str(s)) => s.to_string(),
                            _ => "Unknown".to_string(),
                        };

                        NetworkInfo {
                            name: ssid.clone(),
                            ssid,
                            connection_type,
                            icon: Self::get_wifi_icon(0), // Placeholder icon
                            ip_address: "0.0.0.0".to_string(), // TODO: Retrieve actual IP
                            mac_address: "00:00:00:00:00:00".to_string(), // TODO: Retrieve actual MAC
                            signal_strength: 0, // TODO: Implement signal strength retrieval
                            security_type: WiFiSecurityType::None, // TODO: Implement security type retrieval
                            is_connected: true,
                        }
                    },
                    _ => NetworkInfo::default(),
                }
            },
            _ => NetworkInfo::default(),
        };

        Ok(active_connections)
    }

    pub fn list_wifi_networks(&self) -> Result<Vec<NetworkInfo>> {
        let _network_manager_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        )?;

        // Get all devices
        let _devices_variant = self.proxy.get(
            InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager"), 
            "Devices"
        ).map_err(|e| NetworkError::from(e))?;

        // TODO: Filter and process WiFi devices
        Ok(Vec::new())
    }

    pub fn connect_to_wifi(&self, _config: WiFiConnectionConfig) -> Result<()> {
        let _connection_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        )?;

        // TODO: Implement actual WiFi connection logic
        // This would involve creating a connection and activating it
        Ok(())
    }

    pub fn toggle_network_state(&self, enabled: bool) -> Result<bool> {
        let _network_manager_proxy = zbus::blocking::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        )?;

        // TODO: Implement actual network state toggling
        // This would involve calling the Enable method on NetworkManager
        Ok(enabled)
    }

    pub fn listen_network_changes(&self) -> Result<mpsc::Receiver<NetworkInfo>> {
        let (_tx, rx) = mpsc::channel();
        
        Ok(rx)
    }
}

pub async fn init(app: &AppHandle<tauri::Wry>, _api: PluginApi<tauri::Wry, ()>) -> crate::error::Result<VSKNetworkManager<'static, tauri::Wry>> {
  VSKNetworkManager::new(app.clone()).await
}
