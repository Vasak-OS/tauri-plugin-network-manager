use tauri_plugin_network_manager::{VSKNetworkManager, NetworkInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Crear una instancia del administrador de red
    let network_manager = VSKNetworkManager::new()?;
    
    // Listar redes WiFi disponibles
    println!("Escaneando redes WiFi...");
    match network_manager.list_wifi_networks() {
        Ok(networks) => {
            println!("Redes WiFi encontradas: {}", networks.len());
            for (i, network) in networks.iter().enumerate() {
                println!("{}. {} (Señal: {}%, Seguridad: {:?}, Conectado: {})",
                    i + 1,
                    network.ssid,
                    network.signal_strength,
                    network.security_type,
                    if network.is_connected { "Sí" } else { "No" }
                );
            }
        },
        Err(e) => {
            println!("Error al escanear redes WiFi: {}", e);
        }
    }
    
    Ok(())
}
