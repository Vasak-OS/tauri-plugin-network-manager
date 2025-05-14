use tauri::{State};

use crate::{NetworkError, NetworkInfo, NetworkManagerState, WiFiConnectionConfig};

use crate::error::Result;

/// Get the current network state
#[tauri::command]
pub fn get_network_state(state: State<'_, NetworkManagerState>) -> Result<NetworkInfo> {
    let manager = state.manager.read().map_err(|_| NetworkError::LockError)?;
    
    match manager.as_ref() {
        Some(manager) => {
            let result = manager.get_current_network_state();
            result
        },
        _ => Err(NetworkError::NotInitialized),
    }
}

/// List available WiFi networks
#[tauri::command]
pub fn list_wifi_networks(state: State<'_, NetworkManagerState>) -> Result<Vec<NetworkInfo>> {
    state.inner().list_wifi_networks()
}

/// Connect to a WiFi network
#[tauri::command]
pub fn connect_to_wifi(state: State<'_, NetworkManagerState>, config: WiFiConnectionConfig) -> Result<()> {
    state.inner().connect_to_wifi(config)
}

/// Toggle network on or off
#[tauri::command]
pub fn toggle_network_state(state: State<'_, NetworkManagerState>, enabled: bool) -> Result<bool> {
    state.inner().toggle_network_state(enabled)
}