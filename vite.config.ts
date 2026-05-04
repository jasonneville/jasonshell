import { defineConfig, type PluginOption } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import Terminal from 'vite-plugin-terminal';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(({ command }) => {
  const plugins: PluginOption[] = [svelte()];

  if (command === 'serve') {
    plugins.push(
      Terminal({
        console: 'terminal',
        output: ['terminal', 'console']
      })
    );
  }

  return {
    clearScreen: false,
    plugins,
    server: {
      host: host || false,
      hmr: host
        ? {
            host,
            port: 1421,
            protocol: 'ws'
          }
        : {
            port: 1421
          },
      port: 1420,
      strictPort: true
    }
  };
});


