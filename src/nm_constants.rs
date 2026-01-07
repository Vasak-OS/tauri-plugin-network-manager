// NetworkManager constants
pub const NM_DBUS_SERVICE: &str = "org.freedesktop.NetworkManager";
pub const NM_DBUS_PATH: &str = "/org/freedesktop/NetworkManager";

// Device types
pub const DEVICE_TYPE_ETHERNET: u32 = 1;
pub const DEVICE_TYPE_WIFI: u32 = 2;

// Connection states
pub const CONNECTION_STATE_ACTIVATED: u32 = 2;

// Connectivity states
pub const CONNECTIVITY_NONE: u32 = 1;
pub const CONNECTIVITY_PORTAL: u32 = 2;
pub const CONNECTIVITY_LIMITED: u32 = 3;
pub const CONNECTIVITY_FULL: u32 = 4;

// WiFi security flags
pub const SECURITY_FLAG_NONE: u32 = 0x1;
pub const SECURITY_FLAG_WEP: u32 = 0x2;

// D-Bus interface names
pub const IFACE_NM: &str = "org.freedesktop.NetworkManager";
pub const IFACE_NM_CONNECTION_ACTIVE: &str = "org.freedesktop.NetworkManager.Connection.Active";
pub const IFACE_NM_DEVICE: &str = "org.freedesktop.NetworkManager.Device";
pub const IFACE_NM_DEVICE_WIRELESS: &str = "org.freedesktop.NetworkManager.Device.Wireless";
pub const IFACE_NM_ACCESS_POINT: &str = "org.freedesktop.NetworkManager.AccessPoint";
pub const IFACE_NM_IP4_CONFIG: &str = "org.freedesktop.NetworkManager.IP4Config";
