const COMMANDS: &[&str] = &[
  "get_network_state",
  "list_wifi_networks",
  "rescan_wifi",
  "connect_to_wifi",
  "disconnect_from_wifi",
  "get_saved_wifi_networks",
  "delete_wifi_connection",
  "toggle_network_state",
  "get_wireless_enabled",
  "set_wireless_enabled",
  "is_wireless_available",
  "get_network_stats",
  "get_network_interfaces",
  "list_vpn_profiles",
  "get_vpn_status",
  "connect_vpn",
  "disconnect_vpn",
  "create_vpn_profile",
  "update_vpn_profile",
  "delete_vpn_profile",
];

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .android_path("android")
    .ios_path("ios")
    .build();
}
