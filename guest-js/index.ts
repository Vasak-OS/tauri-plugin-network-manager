import { invoke } from '@tauri-apps/api/core';

export enum NetworkManagerErrorCode {
  NOT_INITIALIZED = 'NOT_INITIALIZED',
  NO_CONNECTION = 'NO_CONNECTION',
  PERMISSION_DENIED = 'PERMISSION_DENIED',
  UNSUPPORTED_SECURITY = 'UNSUPPORTED_SECURITY',
  CONNECTION_FAILED = 'CONNECTION_FAILED',
  NETWORK_NOT_FOUND = 'NETWORK_NOT_FOUND',
  OPERATION_FAILED = 'OPERATION_FAILED',
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