import { invoke } from '@tauri-apps/api/core';

export interface NetworkInfo {
  name: string;
  signal_strength: number;
  icon: string;
  is_connected: boolean;
  ip_address?: string;
  mac_address?: string;
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

export class NetworkManager {
  static async getCurrentNetworkState(): Promise<NetworkInfo> {
    return await invoke('plugin:network-manager|get_network_state');
  }

  static async listWifiNetworks(): Promise<NetworkInfo[]> {
    return await invoke('plugin:network-manager|list_wifi_networks');
  }

  static async connectToWifi(config: WiFiConnectionConfig): Promise<void> {
    return await invoke('plugin:network-manager|connect_to_wifi', {
      ssid: config.ssid,
      password: config.password,
      security_type: config.securityType,
      username: config.username
    });
  }

  static async toggleNetwork(enable: boolean): Promise<void> {
    return await invoke('plugin:network-manager|toggle_network', { enable });
  }
}
