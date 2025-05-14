use tauri::{State};

use crate::{NetworkError, NetworkInfo, NetworkManagerState, WiFiConnectionConfig};
use crate::error::Result;

/// Get the current network state
#[tauri::command]
pub async fn get_network_state(state: State<'_, NetworkManagerState>) -> Result<NetworkInfo> {
    // Clone the Arc to avoid holding the read lock across .await
    let manager_clone = state.manager.clone();
    
    tokio::task::spawn_blocking(move || {
        let manager = manager_clone.read().map_err(|_| NetworkError::LockError)?;
        
        match manager.as_ref() {
            Some(nm) => {
                let result = async move {
                    nm.get_current_network_state().await
                };
                tokio::runtime::Runtime::new().unwrap().block_on(result)
            },
            None => Err(NetworkError::InitializationError),
        }
    }).await.map_err(|_| NetworkError::TaskError)?
}

/// List available WiFi networks
#[tauri::command]
pub async fn list_wifi_networks(state: State<'_, NetworkManagerState>) -> Result<Vec<NetworkInfo>> {
    // Clone the Arc to avoid holding the read lock across .await
    let manager_clone = state.manager.clone();
    
    tokio::task::spawn_blocking(move || {
        let manager = manager_clone.read().map_err(|_| NetworkError::LockError)?;
        
        match manager.as_ref() {
            Some(nm) => {
                let result = async move {
                    nm.list_wifi_networks().await
                };
                tokio::runtime::Runtime::new().unwrap().block_on(result)
            },
            None => Err(NetworkError::InitializationError),
        }
    }).await.map_err(|_| NetworkError::TaskError)?
}

/// Connect to a WiFi network
#[tauri::command]
pub async fn connect_to_wifi(state: State<'_, NetworkManagerState>, config: WiFiConnectionConfig) -> Result<()> {
    // Clone the Arc to avoid holding the read lock across .await
    let manager_clone = state.manager.clone();
    
    tokio::task::spawn_blocking(move || {
        let manager = manager_clone.read().map_err(|_| NetworkError::LockError)?;
        
        match manager.as_ref() {
            Some(nm) => {
                tokio::runtime::Runtime::new().unwrap().block_on(async move {
                    nm.connect_to_wifi(config)
                })
            },
            None => Err(NetworkError::InitializationError),
        }
    }).await.map_err(|_| NetworkError::TaskError)?
}

/// Toggle network on or off
#[tauri::command]
pub async fn toggle_network(state: State<'_, NetworkManagerState>, enable: bool) -> Result<()> {
    // Clone the Arc to avoid holding the read lock across .await
    let manager_clone = state.manager.clone();
    
    tokio::task::spawn_blocking(move || {
        let manager = manager_clone.read().map_err(|_| NetworkError::LockError)?;
        
        match manager.as_ref() {
            Some(nm) => {
                let result = async move {
                    nm.toggle_network(enable).await
                };
                tokio::runtime::Runtime::new().unwrap().block_on(result)
            },
            None => Err(NetworkError::InitializationError),
        }
    }).await.map_err(|_| NetworkError::TaskError)?
}