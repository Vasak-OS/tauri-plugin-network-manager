import { invoke } from '@tauri-apps/api/core';

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

export async function getCurrentNetworkState(): Promise<NetworkInfo> {
  return await invoke('plugin:network-manager|get_network_state');
}

export async function listWifiNetworks(): Promise<NetworkInfo[]> {
  return await invoke('plugin:network-manager|list_wifi_networks');
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

  return await invoke('plugin:network-manager|connect_to_wifi', {
    config: nativeConfig,
  });
}

export async function disconnectFromWifi(): Promise<void> {
  return await invoke('plugin:network-manager|disconnect_from_wifi');
}

export async function getSavedWifiNetworks(): Promise<NetworkInfo[]> {
  return await invoke('plugin:network-manager|get_saved_wifi_networks');
}

export async function deleteWifiConnection(ssid: string): Promise<void> {
  return await invoke('plugin:network-manager|delete_wifi_connection', {
    ssid,
  });
}

export async function toggleNetwork(enabled: boolean): Promise<void> {
  return await invoke('plugin:network-manager|toggle_network_state', { enabled });
}

export async function getWirelessEnabled(): Promise<boolean> {
  return await invoke('plugin:network-manager|get_wireless_enabled');
}

export async function setWirelessEnabled(enabled: boolean): Promise<void> {
  return await invoke('plugin:network-manager|set_wireless_enabled', { enabled });
}

export async function isWirelessAvailable(): Promise<boolean> {
  return await invoke('plugin:network-manager|is_wireless_available');
}

export async function getNetworkStats(): Promise<NetworkStats> {
  return await invoke('plugin:network-manager|get_network_stats');
}

export async function getNetworkInterfaces(): Promise<string[]> {
  return await invoke('plugin:network-manager|get_network_interfaces');
}