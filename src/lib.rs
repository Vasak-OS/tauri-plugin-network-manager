use commands::{connect_to_wifi, get_network_state, list_wifi_networks, toggle_network};
use tauri::{plugin::TauriPlugin, Runtime, Manager};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
pub use desktop::{NetworkInfo, WiFiSecurityType, WiFiConnectionConfig};

#[cfg(desktop)]
mod desktop;

mod commands;
mod error;
mod models;

pub use crate::error::{NetworkError, Result as NetworkResult};

#[derive(Default)]
pub struct NetworkManagerState {
    manager: Arc<RwLock<Option<desktop::VSKNetworkManager<'static, tauri::Wry>>>>,
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
                let rt = tokio::runtime::Runtime::new().unwrap();
                let network_manager: desktop::VSKNetworkManager<'static, tauri::Wry> = rt.block_on(desktop::init(app, _api))?;
                
                app.manage(NetworkManagerState { 
                    manager: Arc::new(RwLock::new(Some(network_manager))) 
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_network_state,
            list_wifi_networks,
            connect_to_wifi,
            toggle_network,
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
