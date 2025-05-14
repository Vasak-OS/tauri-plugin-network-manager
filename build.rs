const COMMANDS: &[&str] = &["get_network_state", "list_wifi_networks", "connect_to_wifi", "toggle_network"];

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .android_path("android")
    .ios_path("ios")
    .build();
}
