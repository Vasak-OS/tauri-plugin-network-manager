use tauri::{AppHandle, Manager};

use crate::{NetworkError, NetworkManagerState};
use crate::models::{
    NetworkInfo, WiFiConnectionConfig, VpnCreateConfig, VpnProfile, VpnStatus, VpnUpdateConfig,
};
use crate::error::Result;

/// Runs NetworkManager work off the caller's thread.
///
/// Every operation here talks to NetworkManager over D-Bus with zbus's blocking
/// API, which parks the thread until the reply arrives. Called straight from an
/// async command that lands on a Tokio worker, and zbus tries to start a
/// runtime inside the one already driving that thread — which does not panic
/// politely, it aborts the whole application. That is what closed the settings
/// window the moment the Wi-Fi page opened.
///
/// A synchronous command is no better: those run on the main thread, so every
/// scan and every connection attempt froze the interface for as long as
/// NetworkManager took to answer.
async fn off_thread<T, F>(work: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| {
            NetworkError::OperationError(format!("network task failed: {error}"))
        })?
}

/// Get the current network state
#[tauri::command]
pub async fn get_network_state(app_handle: AppHandle) -> Result<NetworkInfo>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        let manager = state.manager.read().map_err(|_| NetworkError::LockError)?;

        match manager.as_ref() {
            Some(manager) => {
                let result = manager.get_current_network_state();
                result
            },
            _ => Err(NetworkError::NotInitialized),
        }
    })
    .await
}

/// List available WiFi networks
#[tauri::command]
pub async fn list_wifi_networks(
    app_handle: AppHandle,
    force_refresh: Option<bool>,
    ttl_ms: Option<u64>,
) -> Result<Vec<NetworkInfo>>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.list_wifi_networks(force_refresh.unwrap_or(false), ttl_ms)
    })
    .await
}

/// Trigger a WiFi rescan and return a fresh list
#[tauri::command]
pub async fn rescan_wifi(app_handle: AppHandle) -> Result<Vec<NetworkInfo>>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.rescan_wifi()
    })
    .await
}

/// Connect to a WiFi network
#[tauri::command]
pub async fn connect_to_wifi(app_handle: AppHandle, config: WiFiConnectionConfig) -> Result<()>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.connect_to_wifi(config)?;
        Ok(())
    })
    .await
}

/// Disconnect from the current WiFi network
#[tauri::command]
pub async fn disconnect_from_wifi(app_handle: AppHandle) -> Result<()>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.disconnect_from_wifi()?;
        Ok(())
    })
    .await
}

/// Get saved WiFi networks
#[tauri::command]
pub async fn get_saved_wifi_networks(app_handle: AppHandle) -> Result<Vec<NetworkInfo>>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.get_saved_wifi_networks()
    })
    .await
}

/// Delete a WiFi connection by SSID
#[tauri::command]
pub async fn delete_wifi_connection(app_handle: AppHandle, ssid: String) -> Result<()>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        let deleted = state.delete_wifi_connection(&ssid)?;
        if !deleted {
            return Err(NetworkError::OperationError(format!(
                "No saved WiFi connection found for SSID '{}'",
                ssid
            )));
        }
        Ok(())
    })
    .await
}

/// Toggle network on or off
#[tauri::command]
pub async fn toggle_network_state(app_handle: AppHandle, enabled: bool) -> Result<()>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.toggle_network_state(enabled)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn get_wireless_enabled(app_handle: AppHandle) -> Result<bool>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        Ok(state.get_wireless_enabled()?)
    })
    .await
}

#[tauri::command]
pub async fn set_wireless_enabled(app_handle: AppHandle, enabled: bool) -> Result<()>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        Ok(state.set_wireless_enabled(enabled)?)
    })
    .await
}

#[tauri::command]
pub async fn is_wireless_available(app_handle: AppHandle) -> Result<bool>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        Ok(state.is_wireless_available()?)
    })
    .await
}

/// Get network statistics for the active interface
#[tauri::command]
pub async fn get_network_stats(app_handle: AppHandle) -> Result<crate::models::NetworkStats>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.get_network_stats()
    })
    .await
}

/// Get list of available network interfaces
#[tauri::command]
pub async fn get_network_interfaces() -> Result<Vec<String>>  {
    off_thread(move || {
        crate::network_stats::get_network_interfaces()
            .map_err(|e| NetworkError::from(e))
    })
    .await
}

/// List saved VPN profiles
#[tauri::command]
pub async fn list_vpn_profiles(app_handle: AppHandle) -> Result<Vec<VpnProfile>>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.list_vpn_profiles()
    })
    .await
}

/// Get current VPN status
#[tauri::command]
pub async fn get_vpn_status(app_handle: AppHandle) -> Result<VpnStatus>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.get_vpn_status()
    })
    .await
}

/// Connect VPN by profile UUID
#[tauri::command]
pub async fn connect_vpn(app_handle: AppHandle, uuid: String) -> Result<()>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.connect_vpn(uuid)
    })
    .await
}

/// Disconnect active VPN or specific profile UUID if provided
#[tauri::command]
pub async fn disconnect_vpn(app_handle: AppHandle, uuid: Option<String>) -> Result<()>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.disconnect_vpn(uuid)
    })
    .await
}

/// Create a VPN profile
#[tauri::command]
pub async fn create_vpn_profile(app_handle: AppHandle, config: VpnCreateConfig) -> Result<VpnProfile>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.create_vpn_profile(config)
    })
    .await
}

/// Update a VPN profile
#[tauri::command]
pub async fn update_vpn_profile(app_handle: AppHandle, config: VpnUpdateConfig) -> Result<VpnProfile>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.update_vpn_profile(config)
    })
    .await
}

/// Delete VPN profile by UUID
#[tauri::command]
pub async fn delete_vpn_profile(app_handle: AppHandle, uuid: String) -> Result<()>  {
    off_thread(move || {
        let state = app_handle.state::<NetworkManagerState<tauri::Wry>>();
        state.delete_vpn_profile(uuid)
    })
    .await
}