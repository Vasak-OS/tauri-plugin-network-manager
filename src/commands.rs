use tauri::{AppHandle, Manager};

use crate::{NetworkError, NetworkManagerState};
use crate::models::{NetworkInfo, WiFiConnectionConfig};
use crate::error::Result;

/// Get the current network state
#[tauri::command]
pub async fn get_network_state(app_handle: AppHandle) -> Result<NetworkInfo> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
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
pub async fn list_wifi_networks(app_handle: AppHandle) -> Result<Vec<NetworkInfo>> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
    state.list_wifi_networks()
}

/// Connect to a WiFi network
#[tauri::command]
pub fn connect_to_wifi(app_handle: AppHandle, config: WiFiConnectionConfig) -> Result<()> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
    let _ = state.connect_to_wifi(config);
    Ok(())
}

/// Disconnect from the current WiFi network
#[tauri::command]
pub fn disconnect_from_wifi(app_handle: AppHandle) -> Result<()> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
    let _ = state.disconnect_from_wifi();
    Ok(())
}

/// Get saved WiFi networks
#[tauri::command]
pub async fn get_saved_wifi_networks(app_handle: AppHandle) -> Result<Vec<NetworkInfo>> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
    state.get_saved_wifi_networks()
}

/// Delete a WiFi connection by SSID
#[tauri::command]
pub fn delete_wifi_connection(app_handle: AppHandle, ssid: &str) -> Result<()> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
    let _ = state.delete_wifi_connection(ssid);
    Ok(())
}

/// Toggle network on or off
#[tauri::command]
pub fn toggle_network_state(app_handle: AppHandle, enabled: bool) -> Result<()> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
    let _ = state.toggle_network_state(enabled);
    Ok(())
}

#[tauri::command]
pub fn get_wireless_enabled(app_handle: AppHandle) -> Result<bool> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
    Ok(state.get_wireless_enabled()?)
}

#[tauri::command]
pub fn set_wireless_enabled(app_handle: AppHandle, enabled: bool) -> Result<()> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
    Ok(state.set_wireless_enabled(enabled)?)
}

#[tauri::command]
pub fn is_wireless_available(app_handle: AppHandle) -> Result<bool> {
    let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
    Ok(state.is_wireless_available()?)
}