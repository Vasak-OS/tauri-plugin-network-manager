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

export enum WiFiSecurityType {
  NONE = 'none',
  WEP = 'wep',
  WPA_PSK = 'wpa-psk',
  WPA_EAP = 'wpa-eap',
  WPA2_PSK = 'wpa2-psk',
  WPA3_PSK = 'wpa3-psk'
}

export interface WiFiConnectionConfig {
  ssid: string;
  password?: string;
  securityType: WiFiSecurityType;
  username?: string; // Para WPA-EAP
}

export async function getCurrentNetworkState(): Promise<NetworkInfo> {
  return await invoke('plugin:network-manager|get_network_state');
}

export async function listWifiNetworks(): Promise<NetworkInfo[]> {
  return await invoke('plugin:network-manager|list_wifi_networks');
}

export async function connectToWifi(config: WiFiConnectionConfig): Promise<void> {
  return await invoke('plugin:network-manager|connect_to_wifi', {
    ssid: config.ssid,
    password: config.password,
    security_type: config.securityType,
    username: config.username
  });
}

export async function toggleNetwork(enabled: boolean): Promise<void> {
  return await invoke('plugin:network-manager|toggle_network_state', { enabled });
}
