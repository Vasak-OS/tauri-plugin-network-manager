# Tauri Network Manager Plugin

A Rust-based Tauri plugin for managing network connections on Linux systems using NetworkManager.

## Features

- Get current network state
- List available WiFi networks
- Connect to WiFi networks
- Toggle network connections

## Installation

Add this plugin to your Tauri project by installing the package and registering the plugin.

### Rust (Cargo.toml)

```toml
[dependencies]
tauri-plugin-network-manager = { git = "https://github.com/yourusername/tauri-plugin-network-manager" }
```

### TypeScript

```typescript
import { NetworkManager } from 'tauri-plugin-network-manager';
```

## Usage

### Get Current Network State

```typescript
const networkState = await NetworkManager.getCurrentNetworkState();
console.log(networkState);
// {
//   name: 'MyWiFi',
//   signal_strength: 85,
//   icon: 'wifi-4',
//   is_connected: true,
//   ip_address: '192.168.1.100',
//   mac_address: '00:11:22:33:44:55'
// }
```

### List WiFi Networks

```typescript
const networks = await NetworkManager.listWifiNetworks();
console.log(networks);
```

### Connect to WiFi

```typescript
await NetworkManager.connectToWifi('MyWiFi', 'password123');
```

### Toggle Network

```typescript
await NetworkManager.toggleNetwork(true);  // Enable network
await NetworkManager.toggleNetwork(false); // Disable network
```

## Requirements

- Linux with NetworkManager
- Rust 1.77.2+
- Tauri 2.5+

## License

MIT License
