const COMMANDS: &[&str] = &["get_network_state", "list_wifi_networks", "connect_to_wifi", "disconnect_from_wifi", "get_saved_wifi_networks", "delete_wifi_connection", "toggle_network_state"];

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .android_path("android")
    .ios_path("ios")
    .build();
}
