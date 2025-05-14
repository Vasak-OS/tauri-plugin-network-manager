use tauri::{AppHandle, Runtime, plugin::PluginApi};
use tokio::sync::mpsc;
use zbus::names::InterfaceName;
use zbus::zvariant::Value;
use crate::{models::*, NetworkError};
use crate::error::Result;


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
                            icon: Self::get_wifi_icon(0),
                            ip_address: "0.0.0.0".to_string(), // TODO: Retrieve actual IP
                            mac_address: "00:00:00:00:00:00".to_string(), // TODO: Retrieve actual MAC address
                            signal_strength: 0, // TODO: Retrieve actual signal strength
                            security_type: WiFiSecurityType::None, // TODO: Determine actual security type
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
        let (_tx, rx) = mpsc::channel(10);
        
        Ok(rx)
    }
}

pub async fn init(app: &AppHandle<tauri::Wry>, _api: PluginApi<tauri::Wry, ()>) -> Result<VSKNetworkManager<'static, tauri::Wry>> {
  VSKNetworkManager::new(app.clone()).await
}
