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
});
