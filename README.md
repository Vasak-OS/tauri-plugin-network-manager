# Tauri Network Manager Plugin

Linux-first Tauri plugin to manage network state and Wi-Fi connections through NetworkManager over D-Bus.

## Features

- Read current network state
- List available Wi-Fi networks
- Connect and disconnect Wi-Fi
- List and delete saved Wi-Fi connections
- Enable or disable networking and wireless
- Check wireless adapter availability
- Read network stats and available interfaces
- Listen to network change events (`network-changed`)

## Requirements

- Linux with NetworkManager running
- Tauri 2
- Rust 1.77.2+

## Installation

### Rust dependency

Add the plugin crate to your Tauri app:

```toml
[dependencies]
tauri-plugin-network-manager = { git = "https://github.com/Vasak-OS/tauri-plugin-network-manager" }
```

Register the plugin in your Tauri builder.

### NPM package

Install the JS guest bindings:

```bash
bun add @vasakgroup/plugin-network-manager
```

## Permissions (Tauri capabilities)

The default permission set includes:

- `network-manager:allow-get-network-state`
- `network-manager:allow-list-wifi-networks`
- `network-manager:allow-rescan-wifi`
- `network-manager:allow-connect-to-wifi`
- `network-manager:allow-disconnect-from-wifi`
- `network-manager:allow-get-saved-wifi-networks`
- `network-manager:allow-delete-wifi-connection`
- `network-manager:allow-toggle-network-state`
- `network-manager:allow-get-wireless-enabled`
- `network-manager:allow-set-wireless-enabled`
- `network-manager:allow-is-wireless-available`

## Types exposed in NPM

The package exports these TypeScript types:

- `NetworkInfo`
- `NetworkStats`
- `WiFiSecurityType`
- `WiFiConnectionConfig` (Rust wire format)
- `ConnectToWifiInput` (frontend-friendly format)
- `ListWifiNetworksOptions`
- `NetworkManagerErrorCode`
- `NetworkManagerError`

### Security type values

`WiFiSecurityType` values are:

- `none`
- `wep`
- `wpa-psk`
- `wpa-eap`
- `wpa2-psk`
- `wpa3-psk`

## API reference

### State and scan

- `getCurrentNetworkState(): Promise<NetworkInfo>`
- `listWifiNetworks(options?: ListWifiNetworksOptions): Promise<NetworkInfo[]>`
- `rescanWifi(): Promise<NetworkInfo[]>`
- `getSavedWifiNetworks(): Promise<NetworkInfo[]>`

### Connection management

- `connectToWifi(config: ConnectToWifiInput | WiFiConnectionConfig): Promise<void>`
- `disconnectFromWifi(): Promise<void>`
- `deleteWifiConnection(ssid: string): Promise<void>`

### Radio and networking toggles

- `toggleNetwork(enabled: boolean): Promise<void>`
- `getWirelessEnabled(): Promise<boolean>`
- `setWirelessEnabled(enabled: boolean): Promise<void>`
- `isWirelessAvailable(): Promise<boolean>`

### Stats

- `getNetworkStats(): Promise<NetworkStats>`
- `getNetworkInterfaces(): Promise<string[]>`

## Usage example (TypeScript)

```ts
import {
  connectToWifi,
  deleteWifiConnection,
  disconnectFromWifi,
  getCurrentNetworkState,
  getSavedWifiNetworks,
  listWifiNetworks,
  WiFiSecurityType,
} from '@vasakgroup/plugin-network-manager';

async function run() {
  const state = await getCurrentNetworkState();
  console.log('Current network:', state);

  const available = await listWifiNetworks();
  console.log('Available Wi-Fi:', available.map((n) => n.ssid));

  await connectToWifi({
    ssid: 'OfficeWiFi',
    password: 'secret-password',
    securityType: WiFiSecurityType.WPA2_PSK,
  });

  const saved = await getSavedWifiNetworks();
  console.log('Saved Wi-Fi:', saved.map((n) => n.ssid));

  await disconnectFromWifi();
  await deleteWifiConnection('OldNetwork');
}

run().catch(console.error);
```

## Notes

- `connectToWifi` accepts both formats:
  - Frontend-friendly: `{ securityType: ... }`
  - Rust wire-format: `{ security_type: ... }`
- The wrapper maps frontend input to the command payload expected by Rust.
- `listWifiNetworks` supports cache control:
  - `forceRefresh`: bypasses cache
  - `ttlMs`: cache TTL in milliseconds
- All exported async functions throw a typed `NetworkManagerError` with `code` when invoke fails.

## Testing

- Build package:

```bash
bun run build
```

- Run smoke tests (scan/connect/disconnect wrapper flows):

```bash
bun run test
```
