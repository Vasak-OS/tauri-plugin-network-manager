# Tauri Network Manager Plugin

A Rust-based Tauri plugin for managing network connections on Linux systems using NetworkManager.

## Features

- Get current network state
- Toggle network connections
- Lisen for network state changes

## Future Work

- List available WiFi networks
- Connect to WiFi networks
- Disconnect from WiFi networks

## Installation

Add this plugin to your Tauri project by installing the package and registering the plugin.

### Rust (Cargo.toml)

```toml
[dependencies]
tauri-plugin-network-manager = { git = "https://github.com/Vasak-OS/tauri-plugin-network-manager" }
```

### Node.js

```bash
bun add @vasakgroup/plugin-network-manager  
```

## Usage

```vue
<template>
  <button @click="toggleCurrentNetwork"
    class="p-2 rounded-xl bg-white/50 dark:bg-black/50 hover:bg-white/70 dark:hover:bg-black/70 transition-colors h-[70px] w-[70px]"
    :disabled="isLoading">
    <img :src="networkIconSrc" :alt="networkAlt" class="m-auto w-[50px] h-[50px]" />
  </button>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { getCurrentNetworkState, type NetworkInfo, toggleNetwork } from '@vasakgroup/plugin-network-manager';

let ulisten: Function | null = null;

const networkState = ref<NetworkInfo>(
  {
   ...
  }
);

const toggleCurrentNetwork = async () => {
  try {
    await toggleNetwork(!networkState.value.is_connected);
  } catch (error) {
    console.error('Error toggling network:', error);
  }
};

const getCurrentNetwork = async () => {
  try {
    networkState.value = await getCurrentNetworkState();
  } catch (error) {
    console.error('Error getting current network state:', error);
  }
};

onMounted(async () => {
  await getCurrentNetwork();
  ulisten = await listen<NetworkInfo>('network-changed', async (event) => {
    networkState.value = event.payload;
  });
});

onUnmounted(() => {
  if (ulisten !== null) {
    ulisten();
  }
});
</script>
```

## Requirements

- Linux with NetworkManager
- Rust 1.77.2+
- Tauri 2.5+

## Dependencies

- networkmanager
- dbus
