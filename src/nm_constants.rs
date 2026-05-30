// Connectivity states
pub const CONNECTIVITY_FULL: u32 = 4;

// NM80211ApFlags — Flags property of AccessPoint
// 0x00 = no security (open)
// 0x01 = privacy (authentication required)
pub const AP_FLAGS_NONE: u32 = 0x0;
pub const AP_FLAGS_PRIVACY: u32 = 0x1;

// NM80211ApSecurityFlags — WpaFlags / RsnFlags
pub const SEC_FLAGS_KEY_MGMT_PSK: u32 = 0x00000100;
pub const SEC_FLAGS_KEY_MGMT_802_1X: u32 = 0x00000200;
pub const SEC_FLAGS_KEY_MGMT_SAE: u32 = 0x01000000;

// D-Bus interface names
pub const IFACE_NM: &str = "org.freedesktop.NetworkManager";
pub const IFACE_NM_ACCESS_POINT: &str = "org.freedesktop.NetworkManager.AccessPoint";
