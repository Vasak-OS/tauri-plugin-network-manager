use crate::error::{NetworkError, Result};
use serde::{Deserialize, Serialize};
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use futures::stream::StreamExt;
use zbus::{blocking::Proxy as BlockingProxy, fdo::{self, PropertiesProxy}, zvariant::OwnedObjectPath, Connection};

type NetworkManagerProxyBlocking<'a> = BlockingProxy<'a>;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct NetworkInfo {
    pub name: String,
    pub signal_strength: u8,
    pub icon: String,
    pub is_connected: bool,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WiFiSecurityType {
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
    pub connection: Connection,
    pub proxy: PropertiesProxy<'a>,
    pub app: AppHandle<R>,
}

impl<'a, R: Runtime> VSKNetworkManager<'a, R> {
    pub async fn new(app: AppHandle<R>) -> Result<Self> {
        let connection = zbus::Connection::system().await.unwrap();
        let proxy = fdo::PropertiesProxy::builder(&connection)
            .destination("org.freedesktop.NetworkManager").unwrap()
            .path("/org/freedesktop/NetworkManager").unwrap()
            .interface("org.freedesktop.NetworkManager").unwrap()
            .build().await.unwrap();
        
        Ok(Self {
            connection,
            proxy,
            app,
        })
    }

    pub async fn get_current_network_state(&self) -> Result<NetworkInfo> {
        // Get the active connection path
        let active_connections: Vec<OwnedObjectPath> = self.proxy
            .call("Get", &("org.freedesktop.NetworkManager", "ActiveConnections"))
            .await.map_err(|e| NetworkError::OperationError(e.to_string()))?;
        
        if active_connections.is_empty() {
            return Ok(NetworkInfo {
                name: "No Connection".to_string(),
                signal_strength: 0,
                icon: "wifi-none".to_string(),
                is_connected: false,
                ip_address: None,
                mac_address: None,
            });
        }

        // TODO: Implement detailed network info retrieval
        Ok(NetworkInfo {
            name: "Active Connection".to_string(),
            signal_strength: 50,
            icon: "wifi-medium".to_string(),
            is_connected: true,
            ip_address: None,
            mac_address: None,
        })
    }

    pub async fn list_wifi_networks(&self) -> Result<Vec<NetworkInfo>> {
        // Get list of WiFi devices
        let _devices: Vec<OwnedObjectPath> = self.proxy
            .call("GetDevices", &())
            .await
            .map_err(|e| NetworkError::OperationError(e.to_string()))?;
        
        // TODO: Implement WiFi device parsing
        Ok(vec![])
    }

    pub fn connect_to_wifi(&self, _config: WiFiConnectionConfig) -> Result<()> {
        // TODO: Implement WiFi connection using zbus
        Err(NetworkError::OperationError("Not implemented".to_string()))
    }

    pub async fn toggle_network(&self, enable: bool) -> Result<()> {
        self.proxy
            .call::<_, _, ()>("Enable", &(enable,))
            .await
            .map_err(|e| NetworkError::OperationError(e.to_string()))
    }

    pub async fn listen_network_changes(&self) -> Result<ReceiverStream<NetworkInfo>> {
        let (tx, rx) = mpsc::channel(100);
        
        // Listen to PropertiesChanged signal
        let mut signal_receiver = self.proxy.receive_properties_changed().await.unwrap();
        
        tokio::spawn(async move {
            while let Some(_signal) = signal_receiver.next().await {
                // Process signal and send network info
                // This is a placeholder and needs actual implementation
                let network_info = NetworkInfo::default();
                let _ = tx.send(network_info).await;
            }
        });

        Ok(ReceiverStream::new(rx))
    }

    fn get_wifi_icon(strength: u8) -> String {
        match strength {
            0..=25 => "wifi-weak".to_string(),
            26..=50 => "wifi-medium".to_string(),
            51..=75 => "wifi-strong".to_string(),
            _ => "wifi-full".to_string(),
        }
    }
}

pub async fn init(app: &AppHandle<tauri::Wry>, _api: PluginApi<tauri::Wry, ()>) -> crate::error::Result<VSKNetworkManager<'static, tauri::Wry>> {
  VSKNetworkManager::new(app.clone()).await
}
