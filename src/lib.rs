use commands::{
    connect_to_wifi, connect_vpn, create_vpn_profile, delete_vpn_profile, delete_wifi_connection,
    disconnect_from_wifi, disconnect_vpn, get_network_state, get_vpn_status,
    get_saved_wifi_networks, list_wifi_networks, rescan_wifi, toggle_network_state,
    get_wireless_enabled, list_vpn_profiles, set_wireless_enabled, is_wireless_available,
    update_vpn_profile, get_network_stats, get_network_interfaces
};
pub use models::{
    NetworkInfo, VpnConnectionState, VpnCreateConfig, VpnEventPayload, VpnProfile, VpnStatus,
    VpnUpdateConfig, WiFiConnectionConfig, WiFiSecurityType,
};
use std::result::Result;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Emitter, Manager, Runtime,
};

#[cfg(desktop)]
pub mod desktop;

mod commands;
pub mod error;
pub mod models;
mod nm_constants;
mod nm_helpers;
mod network_stats;

pub use crate::error::{NetworkError, Result as NetworkResult};

pub struct NetworkManagerState<R: Runtime> {
    pub manager: Arc<RwLock<Option<crate::models::VSKNetworkManager<'static, R>>>>,
    pub stats_tracker: Arc<RwLock<Option<crate::network_stats::NetworkStatsTracker>>>,
    pub wifi_networks_cache: Arc<RwLock<Option<WifiNetworksCache>>>,
}

pub struct WifiNetworksCache {
    pub data: Vec<NetworkInfo>,
    pub fetched_at: Instant,
}

impl<R: Runtime> Default for NetworkManagerState<R> {
    fn default() -> Self {
        Self {
            manager: Arc::new(RwLock::new(None)),
            stats_tracker: Arc::new(RwLock::new(None)),
            wifi_networks_cache: Arc::new(RwLock::new(None)),
        }
    }
}

pub fn spawn_network_change_emitter<R: tauri::Runtime>(
    app: AppHandle<R>,
    network_manager: crate::models::VSKNetworkManager<'static, R>,
) {
    let rx = match network_manager.listen_network_changes() {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!("No se pudo escuchar cambios de red: {:?}", e);
            return;
        }
    };

    std::thread::spawn(move || {
        use std::time::{Duration, Instant};
        use std::sync::mpsc::RecvTimeoutError;

        let mut pending_event: Option<crate::models::NetworkInfo> = None;
        let mut pending_vpn_status: Option<crate::models::VpnStatus> = None;
        let mut last_vpn_status: Option<crate::models::VpnStatus> = None;
        let mut debounce_deadline: Option<Instant> = None;
        let debounce_duration = Duration::from_millis(250);

        loop {
            if let Some(deadline) = debounce_deadline {
                let now = Instant::now();
                if now >= deadline {
                    // Timeout reached, emit valid pending event
                    if let Some(info) = pending_event.take() {
                        let _ = app.emit("network-changed", &info);
                    }
                    if let Some(vpn_status) = pending_vpn_status.take() {
                        emit_vpn_events(&app, &network_manager, &mut last_vpn_status, vpn_status);
                    }
                    debounce_deadline = None;
                } else {
                    // Wait for remaining time or new event
                    let timeout = deadline - now;
                    match rx.recv_timeout(timeout) {
                        Ok(info) => {
                            // New event during debounce window: update pending and extend deadline
                            pending_event = Some(info);
                            if let Ok(vpn_status) = network_manager.get_vpn_status() {
                                pending_vpn_status = Some(vpn_status);
                            }
                            debounce_deadline = Some(Instant::now() + debounce_duration);
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            // Loop will handle emission
                        }
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            } else {
                // No pending event, block until one arrives
                match rx.recv() {
                    Ok(info) => {
                        // Leading emission improves perceived latency for UI updates.
                        let _ = app.emit("network-changed", &info);
                        if let Ok(vpn_status) = network_manager.get_vpn_status() {
                            emit_vpn_events(&app, &network_manager, &mut last_vpn_status, vpn_status);
                        }
                        pending_event = None;
                        pending_vpn_status = None;
                        debounce_deadline = Some(Instant::now() + debounce_duration);
                    }
                    Err(_) => break,
                }
            }
        }
    });
}

fn resolve_active_vpn_profile<R: tauri::Runtime>(
    network_manager: &crate::models::VSKNetworkManager<'static, R>,
    status: &VpnStatus,
) -> Option<VpnProfile> {
    let active_uuid = status.active_profile_uuid.as_deref()?;
    let profiles = network_manager.list_vpn_profiles().ok()?;
    profiles.into_iter().find(|profile| profile.uuid == active_uuid)
}

fn emit_vpn_events<R: tauri::Runtime>(
    app: &AppHandle<R>,
    network_manager: &crate::models::VSKNetworkManager<'static, R>,
    last_vpn_status: &mut Option<VpnStatus>,
    status: VpnStatus,
) {
    let previous = last_vpn_status.clone();
    if previous.as_ref() == Some(&status) {
        return;
    }

    let profile = resolve_active_vpn_profile(network_manager, &status);
    let payload = VpnEventPayload {
        status: status.clone(),
        profile,
        reason: None,
    };

    let _ = app.emit("vpn-changed", &payload);

    let previous_state = previous
        .as_ref()
        .map(|s| s.state.clone())
        .unwrap_or(VpnConnectionState::Unknown);

    match status.state {
        VpnConnectionState::Connected => {
            if previous_state != VpnConnectionState::Connected {
                let _ = app.emit("vpn-connected", &payload);
            }
        }
        VpnConnectionState::Disconnected => {
            if previous_state == VpnConnectionState::Connected
                || previous_state == VpnConnectionState::Disconnecting
            {
                let _ = app.emit("vpn-disconnected", &payload);
            }
        }
        VpnConnectionState::Failed => {
            let failed_payload = VpnEventPayload {
                reason: Some("vpn-connection-failed".to_string()),
                ..payload.clone()
            };
            let _ = app.emit("vpn-failed", &failed_payload);
        }
        _ => {}
    }

    *last_vpn_status = Some(status);
}

impl<R: Runtime> NetworkManagerState<R> {
    pub fn new(manager: Option<crate::models::VSKNetworkManager<'static, R>>) -> Self {
        Self {
            manager: Arc::new(RwLock::new(manager)),
            stats_tracker: Arc::new(RwLock::new(None)),
            wifi_networks_cache: Arc::new(RwLock::new(None)),
        }
    }

    pub fn list_wifi_networks(
        &self,
        force_refresh: bool,
        ttl_ms: Option<u64>,
    ) -> Result<Vec<NetworkInfo>, NetworkError> {
        let ttl = Duration::from_millis(ttl_ms.unwrap_or(3000).clamp(250, 30000));

        if !force_refresh {
            let cache = self
                .wifi_networks_cache
                .read()
                .map_err(|_| NetworkError::LockError)?;
            if let Some(cache_entry) = cache.as_ref() {
                if cache_entry.fetched_at.elapsed() <= ttl {
                    return Ok(cache_entry.data.clone());
                }
            }
        }

        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        let networks = match manager.as_ref() {
            Some(manager) => manager.list_wifi_networks(),
            _none => Err(NetworkError::NotInitialized),
        }?;

        let mut cache = self
            .wifi_networks_cache
            .write()
            .map_err(|_| NetworkError::LockError)?;
        *cache = Some(WifiNetworksCache {
            data: networks.clone(),
            fetched_at: Instant::now(),
        });

        Ok(networks)
    }

    pub fn invalidate_wifi_networks_cache(&self) -> Result<(), NetworkError> {
        let mut cache = self
            .wifi_networks_cache
            .write()
            .map_err(|_| NetworkError::LockError)?;
        *cache = None;
        Ok(())
    }

    pub fn rescan_wifi(&self) -> Result<Vec<NetworkInfo>, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.rescan_wifi()?,
            _none => return Err(NetworkError::NotInitialized),
        };
        drop(manager);

        self.invalidate_wifi_networks_cache()?;
        self.list_wifi_networks(true, None)
    }

    pub fn connect_to_wifi(&self, config: WiFiConnectionConfig) -> Result<(), NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.connect_to_wifi(config),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn disconnect_from_wifi(&self) -> Result<(), NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.disconnect_from_wifi(),
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

    pub fn get_wireless_enabled(&self) -> Result<bool, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.get_wireless_enabled().map_err(|e| NetworkError::from(e)),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn set_wireless_enabled(&self, enabled: bool) -> Result<(), NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
             Some(manager) => manager.set_wireless_enabled(enabled).map_err(|e| NetworkError::from(e)),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn is_wireless_available(&self) -> Result<bool, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.is_wireless_available().map_err(|e| NetworkError::from(e)),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn get_network_stats(&self) -> Result<crate::models::NetworkStats, NetworkError> {
        let mut tracker = self.stats_tracker.write().map_err(|_| NetworkError::LockError)?;
        
        // Initialize tracker if not already initialized
        if tracker.is_none() {
            // Get active interface from network manager
            let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
            if let Some(manager) = manager.as_ref() {
                let network_state = manager.get_current_network_state()
                    .map_err(|e| NetworkError::OperationError(e.to_string()))?;
                
                // Try to determine interface name from connection type
                let interface = if network_state.connection_type == "WiFi" {
                    // Try common WiFi interface names
                    crate::network_stats::get_network_interfaces()
                        .ok()
                        .and_then(|interfaces| {
                            interfaces.into_iter()
                                .find(|i| i.starts_with("wl") || i.starts_with("wlan"))
                        })
                        .unwrap_or_else(|| "wlan0".to_string())
                } else {
                    // Try common ethernet interface names
                    crate::network_stats::get_network_interfaces()
                        .ok()
                        .and_then(|interfaces| {
                            interfaces.into_iter()
                                .find(|i| i.starts_with("en") || i.starts_with("eth"))
                        })
                        .unwrap_or_else(|| "eth0".to_string())
                };
                
                *tracker = crate::network_stats::NetworkStatsTracker::new(interface).ok();
            }
        }
        
        // Get stats from tracker
        match tracker.as_mut() {
            Some(t) => t.get_stats().map_err(|e| NetworkError::from(e)),
            None => Err(NetworkError::OperationError("Stats tracker not initialized".to_string())),
        }
    }

    pub fn list_vpn_profiles(&self) -> Result<Vec<VpnProfile>, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.list_vpn_profiles(),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn get_vpn_status(&self) -> Result<VpnStatus, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.get_vpn_status(),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn connect_vpn(&self, uuid: String) -> Result<(), NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.connect_vpn(uuid),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn disconnect_vpn(&self, uuid: Option<String>) -> Result<(), NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.disconnect_vpn(uuid),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn create_vpn_profile(&self, config: VpnCreateConfig) -> Result<VpnProfile, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.create_vpn_profile(config),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn update_vpn_profile(&self, config: VpnUpdateConfig) -> Result<VpnProfile, NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.update_vpn_profile(config),
            _none => Err(NetworkError::NotInitialized),
        }
    }

    pub fn delete_vpn_profile(&self, uuid: String) -> Result<(), NetworkError> {
        let manager = self.manager.read().map_err(|_| NetworkError::LockError)?;
        match manager.as_ref() {
            Some(manager) => manager.delete_vpn_profile(uuid),
            _none => Err(NetworkError::NotInitialized),
        }
    }
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
            rescan_wifi,
            delete_wifi_connection,
            toggle_network_state,
            get_wireless_enabled,
            set_wireless_enabled,
            is_wireless_available,
            get_network_stats,
            get_network_interfaces,
            list_vpn_profiles,
            get_vpn_status,
            connect_vpn,
            disconnect_vpn,
            create_vpn_profile,
            update_vpn_profile,
            delete_vpn_profile,
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

            app.state::<NetworkManagerState<tauri::Wry>>()
                .manager
                .read()
                .map_err(|_| NetworkError::LockError)?
                .as_ref()
                .map(|manager| {
                    // Clone the manager with a 'static lifetime
                    let manager_static: crate::models::VSKNetworkManager<'static, tauri::Wry> =
                        crate::models::VSKNetworkManager {
                            connection: manager.connection.clone(),
                            proxy: manager.proxy.clone(),
                            app: app.clone(),
                        };
                    spawn_network_change_emitter(app.clone(), manager_static);
                });

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
