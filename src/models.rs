use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub name: String,
    pub ssid: String,
    pub connection_type: String,
    pub icon: String,
    pub ip_address: String,
    pub mac_address: String,
    pub signal_strength: u8,
    pub security_type: WiFiSecurityType,
    pub is_connected: bool,
}

impl Default for NetworkInfo {
    fn default() -> Self {
        Self {
            name: String::from("Unknown"),
            ssid: String::from("Unknown"),
            connection_type: String::from("Unknown"),
            icon: String::from("network-offline-symbolic"), // icono por defecto
            ip_address: String::from("0.0.0.0"),
            mac_address: String::from("00:00:00:00:00:00"),
            signal_strength: 0,
            security_type: WiFiSecurityType::None,
            is_connected: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum WiFiSecurityType {
    None,
    Wep,
    WpaPsk,
    WpaEap,
    Wpa2Psk,
    Wpa3Psk,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WiFiNetwork {
    pub ssid: String,
    pub signal_strength: u8,
    pub icon: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WiFiConnectionConfig {
    pub ssid: String,
    pub password: Option<String>,
    pub security_type: WiFiSecurityType,
    pub username: Option<String>,
}

/// Network statistics for bandwidth monitoring
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkStats {
    /// Current download speed in bytes per second
    pub download_speed: u64,
    /// Current upload speed in bytes per second
    pub upload_speed: u64,
    /// Total bytes downloaded since connection
    pub total_downloaded: u64,
    /// Total bytes uploaded since connection
    pub total_uploaded: u64,
    /// Connection duration in seconds
    pub connection_duration: u64,
    /// Network interface name
    pub interface: String,
}

/// Bandwidth data point for historical tracking
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BandwidthPoint {
    /// Timestamp in seconds since epoch
    pub timestamp: u64,
    /// Download speed in bytes per second
    pub download_speed: u64,
    /// Upload speed in bytes per second
    pub upload_speed: u64,
}

// Removed duplicate init function

#[derive(Clone, Debug)]
pub struct VSKNetworkManager<'a, R: Runtime> {
    pub connection: zbus::blocking::Connection,
    pub proxy: zbus::blocking::fdo::PropertiesProxy<'a>,
    pub app: AppHandle<R>,
}
