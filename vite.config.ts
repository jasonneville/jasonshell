import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  clearScreen: false,
  plugins: [svelte()],
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
});
