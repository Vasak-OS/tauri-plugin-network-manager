use std::sync::mpsc;
use std::collections::HashMap;
use tauri::{AppHandle, Runtime, plugin::PluginApi};
use zbus::{names::InterfaceName, zvariant::{Value, OwnedValue}};

use crate::models::*;
use crate::error::{Result, NetworkError};
use crate::network_utils;

impl<R: Runtime> VSKNetworkManager<'static, R> {
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

                        // TODO: Implement actual network info retrieval
                        NetworkInfo {
                            name: "Placeholder Network".to_string(),
                            icon: "network-wireless".to_string(),
                            signal_strength: 0,
                            security_type: WiFiSecurityType::None,
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
        // TODO: Implement actual WiFi network listing
        Ok(vec![])
    }

    pub fn connect_to_wifi(&self, _config: WiFiConnectionConfig) -> Result<()> {
        // TODO: Implement actual WiFi connection logic
        Ok(())
    }

    pub fn toggle_network_state(&self, enabled: bool) -> Result<bool> {
        // TODO: Implement network state toggling
        Ok(enabled)
    }

    pub fn listen_network_changes(&self) -> Result<mpsc::Receiver<NetworkInfo>> {
        // TODO: Implement network change listening
        let (tx, rx) = mpsc::channel();
        Ok(rx)
    }
}

pub fn init(app: &AppHandle<tauri::Wry>, _api: PluginApi<tauri::Wry, ()>) -> Result<VSKNetworkManager<'static, tauri::Wry>> {
    let network_manager = VSKNetworkManager::new(app.clone())?;
    Ok(network_manager)
}
