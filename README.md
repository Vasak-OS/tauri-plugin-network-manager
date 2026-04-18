# Tauri Network Manager Plugin

Linux-first Tauri plugin to manage network state, Wi-Fi and VPN through NetworkManager over D-Bus.

## Features

- Read current network state
- List available Wi-Fi networks
- Connect and disconnect Wi-Fi
- List and delete saved Wi-Fi connections
- Enable or disable networking and wireless
- Check wireless adapter availability
- Read network stats and available interfaces
- List VPN profiles
- Read current VPN status
- Connect and disconnect VPN profiles
- Create, update and delete VPN profiles
- Listen to network and VPN change events (`network-changed`, `vpn-changed`, `vpn-connected`, `vpn-disconnected`, `vpn-failed`)

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
- `network-manager:allow-list-vpn-profiles`
- `network-manager:allow-get-vpn-status`

Mutating VPN operations are intentionally excluded from default permissions.

To enable VPN management operations, add the dedicated permission set:

- `network-manager:vpn_management`

This set includes:

- `network-manager:allow-connect-vpn`
- `network-manager:allow-disconnect-vpn`
- `network-manager:allow-create-vpn-profile`
- `network-manager:allow-update-vpn-profile`
- `network-manager:allow-delete-vpn-profile`

## Types exposed in NPM

The package exports these TypeScript types:

- `NetworkInfo`
- `NetworkStats`
- `WiFiSecurityType`
- `WiFiConnectionConfig` (Rust wire format)
- `ConnectToWifiInput` (frontend-friendly format)
- `ListWifiNetworksOptions`
- `VpnType`
- `VpnConnectionState`
- `VpnProfile`
- `VpnStatus`
- `VpnEventPayload`
- `VpnCreateInput`
- `VpnUpdateInput`
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

### VPN type values

`VpnType` values are:

- `open-vpn`
- `wire-guard`
- `l2tp`
- `pptp`
- `sstp`
- `ikev2`
- `fortisslvpn`
- `open-connect`
- `generic`

## API reference

### State and scan

- `getCurrentNetworkState(): Promise<NetworkInfo>`
- `listWifiNetworks(options?: ListWifiNetworksOptions): Promise<NetworkInfo[]>`
- `rescanWifi(): Promise<NetworkInfo[]>`
- `getSavedWifiNetworks(): Promise<NetworkInfo[]>`

### Wi-Fi connection management

- `connectToWifi(config: ConnectToWifiInput | WiFiConnectionConfig): Promise<void>`
- `disconnectFromWifi(): Promise<void>`
- `deleteWifiConnection(ssid: string): Promise<void>`

### VPN profile and connection management

- `listVpnProfiles(): Promise<VpnProfile[]>`
- `getVpnStatus(): Promise<VpnStatus>`
- `connectVpn(uuid: string): Promise<void>`
- `disconnectVpn(uuid?: string): Promise<void>`
- `createVpnProfile(config: VpnCreateInput): Promise<VpnProfile>`
- `updateVpnProfile(config: VpnUpdateInput): Promise<VpnProfile>`
- `deleteVpnProfile(uuid: string): Promise<void>`

### Radio and networking toggles

- `toggleNetwork(enabled: boolean): Promise<void>`
- `getWirelessEnabled(): Promise<boolean>`
- `setWirelessEnabled(enabled: boolean): Promise<void>`
- `isWirelessAvailable(): Promise<boolean>`

### Stats

- `getNetworkStats(): Promise<NetworkStats>`
- `getNetworkInterfaces(): Promise<string[]>`

## Usage examples (TypeScript)

### Wi-Fi

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

async function wifiFlow() {
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

wifiFlow().catch(console.error);
```

### VPN

```ts
import {
  connectVpn,
  createVpnProfile,
  disconnectVpn,
  getVpnStatus,
  listVpnProfiles,
  updateVpnProfile,
  deleteVpnProfile,
} from '@vasakgroup/plugin-network-manager';

async function vpnFlow() {
  const profile = await createVpnProfile({
    id: 'Office VPN',
    vpn_type: 'wire-guard',
    autoconnect: false,
    settings: {
      mtu: '1420',
      remote: 'vpn.example.com:51820',
    },
    secrets: {
      private_key: '***',
    },
  });

  console.log('Created profile:', profile);

  const profiles = await listVpnProfiles();
  console.log('Profiles:', profiles.map((p) => `${p.id} (${p.uuid})`));

  await connectVpn(profile.uuid);
  console.log('VPN status after connect:', await getVpnStatus());

  const updated = await updateVpnProfile({
    uuid: profile.uuid,
    autoconnect: true,
  });
  console.log('Updated profile:', updated);

  await disconnectVpn(profile.uuid);
  await deleteVpnProfile(profile.uuid);
}

vpnFlow().catch(console.error);
```

### Listening to VPN events

```ts
import { listen } from '@tauri-apps/api/event';
import type { VpnEventPayload } from '@vasakgroup/plugin-network-manager';

const unlistenChanged = await listen<VpnEventPayload>('vpn-changed', (event) => {
  console.log('VPN changed:', event.payload.status.state);
});

const unlistenConnected = await listen<VpnEventPayload>('vpn-connected', (event) => {
  console.log('VPN connected:', event.payload.profile?.id);
});

const unlistenFailed = await listen<VpnEventPayload>('vpn-failed', (event) => {
  console.error('VPN failed:', event.payload.reason);
});

// call unlistenChanged(), unlistenConnected(), unlistenFailed() when no longer needed
```

## Typed errors

All exported async functions throw a typed `NetworkManagerError` with a `code` in case of invoke failure.

Common Wi-Fi and generic codes:

- `NOT_INITIALIZED`
- `NO_CONNECTION`
- `PERMISSION_DENIED`
- `UNSUPPORTED_SECURITY`
- `CONNECTION_FAILED`
- `NETWORK_NOT_FOUND`
- `OPERATION_FAILED`

VPN-specific codes:

- `VPN_PROFILE_NOT_FOUND`
- `VPN_ALREADY_CONNECTED`
- `VPN_AUTH_FAILED`
- `VPN_INVALID_CONFIG`
- `VPN_ACTIVATION_FAILED`
- `VPN_PLUGIN_UNAVAILABLE`
- `VPN_NOT_ACTIVE`

Fallback code:

- `UNKNOWN`

## Package exports and types

The package is published with ESM + CJS bundles and declaration files:

- `main`: `./dist-js/index.cjs`
- `module`: `./dist-js/index.js`
- `types`: `./dist-js/index.d.ts`
- `exports.types`: `./dist-js/index.d.ts`
- `exports.import`: `./dist-js/index.js`
- `exports.require`: `./dist-js/index.cjs`

## Notes

- `connectToWifi` accepts both formats:
  - frontend-friendly: `{ securityType: ... }`
  - Rust wire-format: `{ security_type: ... }`
- The wrapper maps frontend input to the command payload expected by Rust.
- `listWifiNetworks` supports cache control:
  - `forceRefresh`: bypasses cache
  - `ttlMs`: cache TTL in milliseconds

## Testing and build

Build package:

```bash
bun run build
```

Run smoke tests:

```bash
bun run test
```
