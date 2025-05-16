use commands::{
    connect_to_wifi, delete_wifi_connection, disconnect_from_wifi, get_network_state,
    get_saved_wifi_networks, list_wifi_networks, toggle_network_state,
};
pub use models::{NetworkInfo, WiFiConnectionConfig, WiFiSecurityType};
use serde::{Deserialize, Serialize};
use std::result::Result;
use std::sync::{Arc, RwLock};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(desktop)]
pub mod desktop;

mod commands;
pub mod error;
pub mod models;

pub use crate::error::{NetworkError, Result as NetworkResult};

#[derive(Default)]
pub struct NetworkManagerState<R: Runtime> {
    pub manager: Arc<RwLock<Option<crate::models::VSKNetworkManager<'static, R>>>>,
}

impl<R: Runtime> NetworkManagerState<R> {
    pub fn new(manager: Option<crate::models::VSKNetworkManager<'static, R>>) -> Self {
        Self {
            manager: Arc::new(RwLock::new(manager)),
        }
    }

    pub fn list_wifi_networks(&self) -> Result<Vec<NetworkInfo>, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.list_wifi_networks(),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub async fn connect_to_wifi(&self, config: WiFiConnectionConfig) -> Result<(), NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.connect_to_wifi(config).await,
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub async fn disconnect_from_wifi(&self) -> Result<(), NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.disconnect_from_wifi().await,
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn get_saved_wifi_networks(&self) -> Result<Vec<NetworkInfo>, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.get_saved_wifi_networks(),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn delete_wifi_connection(&self, ssid: &str) -> Result<bool, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.delete_wifi_connection(ssid),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn toggle_network_state(&self, enabled: bool) -> Result<bool, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.toggle_network_state(enabled),
            _none => Err(NetworkError::NotInitialized),
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
    Builder::new("network-manager")
        .invoke_handler(tauri::generate_handler![
            get_network_state,
            list_wifi_networks,
            connect_to_wifi,
            disconnect_from_wifi,
            get_saved_wifi_networks,
            delete_wifi_connection,
            toggle_network_state,
        ])
        .setup(|app, _api| -> Result<(), Box<dyn std::error::Error>> {
            #[cfg(desktop)]
            // Removed tokio runtime initialization
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let network_manager = rt.block_on(async { crate::desktop::init(&app, _api).await })?;

            app.manage(NetworkManagerState::<tauri::Wry>::new(Some(
                network_manager,
            )));

            Ok(())
        })
        .build()
}

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the network-manager APIs.
pub trait NetworkManagerExt<R: Runtime> {
    fn network_manager(&self) -> Option<crate::models::VSKNetworkManager<'static, R>>;
}

impl<R: Runtime + Clone, T: Manager<R>> NetworkManagerExt<R> for T {
    fn network_manager(&self) -> Option<crate::models::VSKNetworkManager<'static, R>> {
        self.try_state::<NetworkManagerState<R>>()
            .and_then(|state| {
                state.manager.read().ok().and_then(|m| {
                    m.as_ref().map(|x| crate::models::VSKNetworkManager {
                        connection: x.connection.clone(),
                        proxy: x.proxy.clone(),
                        app: self.app_handle().clone(),
                    })
                })
            })
    }
}
