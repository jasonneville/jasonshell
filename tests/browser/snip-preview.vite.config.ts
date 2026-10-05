import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Isolated frontend-only evidence server. Never launches Tauri/native shell.
export default defineConfig({
  plugins: [svelte()],
  server: { host: '127.0.0.1', port: 1439, strictPort: true, hmr: false }
});
