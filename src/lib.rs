use commands::{connect_to_wifi, get_network_state, list_wifi_networks};
use tauri::{plugin::TauriPlugin, Manager, Runtime};
use serde::{Deserialize, Serialize};
use std::result::Result;
use std::sync::{Arc, RwLock};
// Removed unused import
pub use desktop::{NetworkInfo, WiFiSecurityType, WiFiConnectionConfig};

#[cfg(desktop)]
mod desktop;

mod commands;
mod error;
mod models;

pub use crate::error::{NetworkError, Result as NetworkResult};

#[derive(Default)]
pub struct NetworkManagerState {
    pub manager: Arc<RwLock<Option<desktop::VSKNetworkManager<'static, tauri::Wry>>>>,
}

impl NetworkManagerState {
    pub fn new(manager: Option<desktop::VSKNetworkManager<'static, tauri::Wry>>) -> Self {
        Self {
            manager: Arc::new(RwLock::new(manager)),
        }
    }

    pub fn list_wifi_networks(&self) -> Result<Vec<NetworkInfo>, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.list_wifi_networks(),
            _ => Err(NetworkError::NotInitialized),
        }
    }

    pub fn connect_to_wifi(&self, config: WiFiConnectionConfig) -> Result<(), NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.connect_to_wifi(config),
            _ => Err(NetworkError::NotInitialized),
        }
    }

    pub fn toggle_network_state(&self, enabled: bool) -> Result<bool, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.toggle_network_state(enabled),
            _ => Err(NetworkError::NotInitialized),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct NetworkRequest {
    ssid: String,
    password: Option<String>,
    security_type: WiFiSecurityType,
    username: Option<String>,
}

/// Initializes the plugin.
pub fn init() -> TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("network-manager")
        .setup(|app, _api| -> Result<(), Box<dyn std::error::Error>> {
            #[cfg(desktop)]
            {
                // Removed tokio runtime initialization
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()?;
let network_manager = rt.block_on(async {
                    desktop::init(&app, _api).await
                })?;
                
                app.manage(NetworkManagerState::new(Some(network_manager)));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_network_state,
            list_wifi_networks,
            connect_to_wifi,

        ])
        .build()
}


/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the network-manager APIs.
pub trait NetworkManagerExt<R: Runtime> {
    fn network_manager(&self) -> Option<desktop::VSKNetworkManager<'static, R>>;
}

impl<R: Runtime + Clone, T: Manager<R>> NetworkManagerExt<R> for T {
    fn network_manager(&self) -> Option<desktop::VSKNetworkManager<'static, R>> {
        self.try_state::<NetworkManagerState>()
            .and_then(|state| state.manager.read().ok().and_then(|m| {
                m.as_ref().map(|x| desktop::VSKNetworkManager {
                    connection: x.connection.clone(),
                    proxy: x.proxy.clone(),
                    app: self.app_handle().clone(),
                })
            }))
    }
}
