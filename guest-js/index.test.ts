import { beforeEach, describe, expect, it, mock } from 'bun:test';

const invokeMock = mock((_: string, __?: Record<string, unknown>) =>
  Promise.resolve(undefined),
);

mock.module('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

describe('network-manager guest-js smoke', () => {
  beforeEach(() => {
    invokeMock.mockClear();
  });

  it('calls rescan command', async () => {
    invokeMock.mockResolvedValueOnce([]);
    const mod = await import('./index');

    await mod.rescanWifi();

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|rescan_wifi', undefined);
  });

  it('calls list with cache options', async () => {
    invokeMock.mockResolvedValueOnce([]);
    const mod = await import('./index');

    await mod.listWifiNetworks({ forceRefresh: true, ttlMs: 1200 });

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|list_wifi_networks', {
      force_refresh: true,
      ttl_ms: 1200,
    });
  });

  it('maps connect input to native payload', async () => {
    const mod = await import('./index');

    await mod.connectToWifi({
      ssid: 'MyWiFi',
      password: 'secret',
      securityType: mod.WiFiSecurityType.WPA2_PSK,
    });

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|connect_to_wifi', {
      config: {
        ssid: 'MyWiFi',
        password: 'secret',
        security_type: 'wpa2-psk',
        username: undefined,
      },
    });
  });

  it('calls disconnect command', async () => {
    const mod = await import('./index');

    await mod.disconnectFromWifi();

    expect(invokeMock).toHaveBeenCalledWith(
      'plugin:network-manager|disconnect_from_wifi',
      undefined,
    );
  });

  it('returns typed permission error', async () => {
    invokeMock.mockRejectedValueOnce(new Error('Permission denied while scanning'));
    const mod = await import('./index');

    await expect(mod.rescanWifi()).rejects.toMatchObject({
      name: 'NetworkManagerError',
      code: mod.NetworkManagerErrorCode.PERMISSION_DENIED,
    });
  });

  it('calls list vpn profiles command', async () => {
    invokeMock.mockResolvedValueOnce([]);
    const mod = await import('./index');

    await mod.listVpnProfiles();

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|list_vpn_profiles', undefined);
  });

  it('calls connect vpn command', async () => {
    const mod = await import('./index');

    await mod.connectVpn('vpn-uuid-1');

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|connect_vpn', {
      uuid: 'vpn-uuid-1',
    });
  });

  it('calls disconnect vpn command with uuid', async () => {
    const mod = await import('./index');

    await mod.disconnectVpn('vpn-uuid-1');

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|disconnect_vpn', {
      uuid: 'vpn-uuid-1',
    });
  });

  it('calls disconnect vpn command without uuid', async () => {
    const mod = await import('./index');

    await mod.disconnectVpn();

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|disconnect_vpn', {
      uuid: undefined,
    });
  });

  it('calls get vpn status command', async () => {
    const statusPayload = {
      state: 'connected',
      active_profile_id: 'Office',
      active_profile_uuid: 'vpn-uuid-1',
      active_profile_name: 'Office VPN',
      ip_address: '10.0.0.5',
      gateway: '10.0.0.1',
      since_unix_ms: 12345,
    };

    invokeMock.mockResolvedValueOnce(statusPayload);
    const mod = await import('./index');

    const status = await mod.getVpnStatus();

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|get_vpn_status', undefined);
    expect(status).toEqual(statusPayload);
  });

  it('calls create vpn profile command', async () => {
    const createdProfile = {
      id: 'Office',
      uuid: 'vpn-uuid-1',
      vpn_type: 'wire-guard',
      autoconnect: true,
      editable: true,
    };

    invokeMock.mockResolvedValueOnce(createdProfile);
    const mod = await import('./index');

    const config = {
      id: 'Office',
      vpn_type: 'wire-guard' as const,
      autoconnect: true,
      settings: {
        mtu: '1420',
      },
      secrets: {
        private_key: 'abc',
      },
    };

    const profile = await mod.createVpnProfile(config);

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|create_vpn_profile', {
      config,
    });
    expect(profile).toEqual(createdProfile);
  });

  it('calls update vpn profile command', async () => {
    const updatedProfile = {
      id: 'Office Updated',
      uuid: 'vpn-uuid-1',
      vpn_type: 'wire-guard',
      autoconnect: false,
      editable: true,
    };

    invokeMock.mockResolvedValueOnce(updatedProfile);
    const mod = await import('./index');

    const config = {
      uuid: 'vpn-uuid-1',
      id: 'Office Updated',
      autoconnect: false,
      settings: {
        mtu: '1380',
      },
    };

    const profile = await mod.updateVpnProfile(config);

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|update_vpn_profile', {
      config,
    });
    expect(profile).toEqual(updatedProfile);
  });

  it('calls delete vpn profile command', async () => {
    const mod = await import('./index');

    await mod.deleteVpnProfile('vpn-uuid-1');

    expect(invokeMock).toHaveBeenCalledWith('plugin:network-manager|delete_vpn_profile', {
      uuid: 'vpn-uuid-1',
    });
  });

  it('returns typed vpn profile not found error', async () => {
    invokeMock.mockRejectedValueOnce(new Error('VPN profile not found: vpn-uuid-1'));
    const mod = await import('./index');

    await expect(mod.connectVpn('vpn-uuid-1')).rejects.toMatchObject({
      name: 'NetworkManagerError',
      code: mod.NetworkManagerErrorCode.VPN_PROFILE_NOT_FOUND,
    });
  });

  it('maps vpn typed errors', async () => {
    const mod = await import('./index');

    invokeMock.mockRejectedValueOnce(new Error('VPN already connected: vpn-uuid-1'));
    await expect(mod.connectVpn('vpn-uuid-1')).rejects.toMatchObject({
      code: mod.NetworkManagerErrorCode.VPN_ALREADY_CONNECTED,
    });

    invokeMock.mockRejectedValueOnce(new Error('VPN authentication failed'));
    await expect(mod.connectVpn('vpn-uuid-1')).rejects.toMatchObject({
      code: mod.NetworkManagerErrorCode.VPN_AUTH_FAILED,
    });

    invokeMock.mockRejectedValueOnce(new Error('Invalid VPN configuration'));
    await expect(mod.createVpnProfile({ id: 'x', vpn_type: 'generic' })).rejects.toMatchObject({
      code: mod.NetworkManagerErrorCode.VPN_INVALID_CONFIG,
    });

    invokeMock.mockRejectedValueOnce(new Error('VPN activation failed'));
    await expect(mod.connectVpn('vpn-uuid-1')).rejects.toMatchObject({
      code: mod.NetworkManagerErrorCode.VPN_ACTIVATION_FAILED,
    });

    invokeMock.mockRejectedValueOnce(new Error('VPN plugin unavailable'));
    await expect(mod.connectVpn('vpn-uuid-1')).rejects.toMatchObject({
      code: mod.NetworkManagerErrorCode.VPN_PLUGIN_UNAVAILABLE,
    });

    invokeMock.mockRejectedValueOnce(new Error('No VPN active'));
    await expect(mod.disconnectVpn()).rejects.toMatchObject({
      code: mod.NetworkManagerErrorCode.VPN_NOT_ACTIVE,
    });
  });
});
