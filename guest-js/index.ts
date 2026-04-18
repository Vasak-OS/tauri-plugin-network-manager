import { invoke } from '@tauri-apps/api/core';

export enum NetworkManagerErrorCode {
  NOT_INITIALIZED = 'NOT_INITIALIZED',
  NO_CONNECTION = 'NO_CONNECTION',
  PERMISSION_DENIED = 'PERMISSION_DENIED',
  UNSUPPORTED_SECURITY = 'UNSUPPORTED_SECURITY',
  CONNECTION_FAILED = 'CONNECTION_FAILED',
  NETWORK_NOT_FOUND = 'NETWORK_NOT_FOUND',
  OPERATION_FAILED = 'OPERATION_FAILED',
  VPN_PROFILE_NOT_FOUND = 'VPN_PROFILE_NOT_FOUND',
  VPN_ALREADY_CONNECTED = 'VPN_ALREADY_CONNECTED',
  VPN_AUTH_FAILED = 'VPN_AUTH_FAILED',
  VPN_INVALID_CONFIG = 'VPN_INVALID_CONFIG',
  VPN_ACTIVATION_FAILED = 'VPN_ACTIVATION_FAILED',
  VPN_PLUGIN_UNAVAILABLE = 'VPN_PLUGIN_UNAVAILABLE',
  VPN_NOT_ACTIVE = 'VPN_NOT_ACTIVE',
  UNKNOWN = 'UNKNOWN',
}

export interface NetworkManagerError extends Error {
  code: NetworkManagerErrorCode;
  details?: unknown;
}

export interface NetworkInfo {
  name: string;
  ssid: string;
  connection_type: string;
  icon: string;
  ip_address: string;
  mac_address: string;
  signal_strength: number;
  security_type: WiFiSecurityType;
  is_connected: boolean;
}

export interface NetworkStats {
  download_speed: number;
  upload_speed: number;
  total_downloaded: number;
  total_uploaded: number;
  connection_duration: number;
  interface: string;
}

export enum WiFiSecurityType {
  NONE = 'none',
  WEP = 'wep',
  WPA_PSK = 'wpa-psk',
  WPA_EAP = 'wpa-eap',
  WPA2_PSK = 'wpa2-psk',
  WPA3_PSK = 'wpa3-psk'
}

/**
 * Wire-format expected by the Rust command `connect_to_wifi`.
 */
export interface WiFiConnectionConfig {
  ssid: string;
  password?: string;
  security_type: WiFiSecurityType;
  username?: string;
}

/**
 * Ergonomic input for frontend code.
 * This is converted to WiFiConnectionConfig before invoking Rust.
 */
export interface ConnectToWifiInput {
  ssid: string;
  password?: string;
  securityType: WiFiSecurityType;
  username?: string;
}

export interface ListWifiNetworksOptions {
  forceRefresh?: boolean;
  ttlMs?: number;
}

export type VpnType =
  | 'open-vpn'
  | 'wire-guard'
  | 'l2tp'
  | 'pptp'
  | 'sstp'
  | 'ikev2'
  | 'fortisslvpn'
  | 'open-connect'
  | 'generic';

export type VpnConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'disconnecting'
  | 'failed'
  | 'unknown';

export interface VpnProfile {
  id: string;
  uuid: string;
  vpn_type: VpnType;
  interface_name?: string;
  autoconnect: boolean;
  editable: boolean;
  last_error?: string;
}

export interface VpnStatus {
  state: VpnConnectionState;
  active_profile_id?: string;
  active_profile_uuid?: string;
  active_profile_name?: string;
  ip_address?: string;
  gateway?: string;
  since_unix_ms?: number;
}

export interface VpnEventPayload {
  status: VpnStatus;
  profile?: VpnProfile;
  reason?: string;
}

export interface VpnCreateInput {
  id: string;
  vpn_type: VpnType;
  autoconnect?: boolean;
  username?: string;
  password?: string;
  gateway?: string;
  ca_cert_path?: string;
  user_cert_path?: string;
  private_key_path?: string;
  private_key_password?: string;
  settings?: Record<string, string>;
  secrets?: Record<string, string>;
}

export interface VpnUpdateInput {
  uuid: string;
  id?: string;
  autoconnect?: boolean;
  username?: string;
  password?: string;
  gateway?: string;
  ca_cert_path?: string;
  user_cert_path?: string;
  private_key_path?: string;
  private_key_password?: string;
  settings?: Record<string, string>;
  secrets?: Record<string, string>;
}

function normalizeInvokeError(error: unknown): NetworkManagerError {
  const rawMessage =
    typeof error === 'string'
      ? error
      : error && typeof error === 'object' && 'message' in error
      ? String((error as { message?: unknown }).message)
      : 'Unknown network-manager error';

  const message = rawMessage.toLowerCase();

  let code = NetworkManagerErrorCode.UNKNOWN;
  if (message.includes('not initialized')) {
    code = NetworkManagerErrorCode.NOT_INITIALIZED;
  } else if (message.includes('no network connection')) {
    code = NetworkManagerErrorCode.NO_CONNECTION;
  } else if (message.includes('permission denied') || message.includes('not allowed')) {
    code = NetworkManagerErrorCode.PERMISSION_DENIED;
  } else if (message.includes('unsupported wifi security type')) {
    code = NetworkManagerErrorCode.UNSUPPORTED_SECURITY;
  } else if (message.includes('connection failed') || message.includes('addandactivateconnection failed')) {
    code = NetworkManagerErrorCode.CONNECTION_FAILED;
  } else if (message.includes('no saved wifi connection found')) {
    code = NetworkManagerErrorCode.NETWORK_NOT_FOUND;
  } else if (message.includes('operation error') || message.includes('network operation failed')) {
    code = NetworkManagerErrorCode.OPERATION_FAILED;
  } else if (message.includes('vpn profile not found')) {
    code = NetworkManagerErrorCode.VPN_PROFILE_NOT_FOUND;
  } else if (message.includes('vpn already connected')) {
    code = NetworkManagerErrorCode.VPN_ALREADY_CONNECTED;
  } else if (message.includes('vpn authentication failed')) {
    code = NetworkManagerErrorCode.VPN_AUTH_FAILED;
  } else if (message.includes('invalid vpn configuration')) {
    code = NetworkManagerErrorCode.VPN_INVALID_CONFIG;
  } else if (message.includes('vpn activation failed')) {
    code = NetworkManagerErrorCode.VPN_ACTIVATION_FAILED;
  } else if (message.includes('vpn plugin unavailable')) {
    code = NetworkManagerErrorCode.VPN_PLUGIN_UNAVAILABLE;
  } else if (message.includes('no vpn active')) {
    code = NetworkManagerErrorCode.VPN_NOT_ACTIVE;
  }

  const typedError = new Error(rawMessage) as NetworkManagerError;
  typedError.name = 'NetworkManagerError';
  typedError.code = code;
  typedError.details = error;
  return typedError;
}

async function invokeWithTypedError<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeInvokeError(error);
  }
}

export async function getCurrentNetworkState(): Promise<NetworkInfo> {
  return await invokeWithTypedError<NetworkInfo>('plugin:network-manager|get_network_state');
}

export async function listWifiNetworks(
  options: ListWifiNetworksOptions = {},
): Promise<NetworkInfo[]> {
  return await invokeWithTypedError<NetworkInfo[]>('plugin:network-manager|list_wifi_networks', {
    force_refresh: options.forceRefresh,
    ttl_ms: options.ttlMs,
  });
}

export async function rescanWifi(): Promise<NetworkInfo[]> {
  return await invokeWithTypedError<NetworkInfo[]>('plugin:network-manager|rescan_wifi');
}

function toNativeWiFiConnectionConfig(
  config: ConnectToWifiInput | WiFiConnectionConfig,
): WiFiConnectionConfig {
  if ('security_type' in config) {
    return config;
  }

  return {
    ssid: config.ssid,
    password: config.password,
    security_type: config.securityType,
    username: config.username,
  };
}

export async function connectToWifi(
  config: ConnectToWifiInput | WiFiConnectionConfig,
): Promise<void> {
  const nativeConfig = toNativeWiFiConnectionConfig(config);

  return await invokeWithTypedError<void>('plugin:network-manager|connect_to_wifi', {
    config: nativeConfig,
  });
}

export async function disconnectFromWifi(): Promise<void> {
  return await invokeWithTypedError<void>('plugin:network-manager|disconnect_from_wifi');
}

export async function getSavedWifiNetworks(): Promise<NetworkInfo[]> {
  return await invokeWithTypedError<NetworkInfo[]>('plugin:network-manager|get_saved_wifi_networks');
}

export async function deleteWifiConnection(ssid: string): Promise<void> {
  return await invokeWithTypedError<void>('plugin:network-manager|delete_wifi_connection', {
    ssid,
  });
}

export async function toggleNetwork(enabled: boolean): Promise<void> {
  return await invokeWithTypedError<void>('plugin:network-manager|toggle_network_state', {
    enabled,
  });
}

export async function getWirelessEnabled(): Promise<boolean> {
  return await invokeWithTypedError<boolean>('plugin:network-manager|get_wireless_enabled');
}

export async function setWirelessEnabled(enabled: boolean): Promise<void> {
  return await invokeWithTypedError<void>('plugin:network-manager|set_wireless_enabled', {
    enabled,
  });
}

export async function isWirelessAvailable(): Promise<boolean> {
  return await invokeWithTypedError<boolean>('plugin:network-manager|is_wireless_available');
}

export async function getNetworkStats(): Promise<NetworkStats> {
  return await invokeWithTypedError<NetworkStats>('plugin:network-manager|get_network_stats');
}

export async function getNetworkInterfaces(): Promise<string[]> {
  return await invokeWithTypedError<string[]>('plugin:network-manager|get_network_interfaces');
}

export async function listVpnProfiles(): Promise<VpnProfile[]> {
  return await invokeWithTypedError<VpnProfile[]>('plugin:network-manager|list_vpn_profiles');
}

export async function getVpnStatus(): Promise<VpnStatus> {
  return await invokeWithTypedError<VpnStatus>('plugin:network-manager|get_vpn_status');
}

export async function connectVpn(uuid: string): Promise<void> {
  return await invokeWithTypedError<void>('plugin:network-manager|connect_vpn', {
    uuid,
  });
}

export async function disconnectVpn(uuid?: string): Promise<void> {
  return await invokeWithTypedError<void>('plugin:network-manager|disconnect_vpn', {
    uuid,
  });
}

export async function createVpnProfile(config: VpnCreateInput): Promise<VpnProfile> {
  return await invokeWithTypedError<VpnProfile>('plugin:network-manager|create_vpn_profile', {
    config,
  });
}

export async function updateVpnProfile(config: VpnUpdateInput): Promise<VpnProfile> {
  return await invokeWithTypedError<VpnProfile>('plugin:network-manager|update_vpn_profile', {
    config,
  });
}

export async function deleteVpnProfile(uuid: string): Promise<void> {
  return await invokeWithTypedError<void>('plugin:network-manager|delete_vpn_profile', {
    uuid,
  });
}