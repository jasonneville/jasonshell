import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
const boundary = fileURLToPath(new URL('./tauri-boundary.mjs', import.meta.url));
export default defineConfig({
  plugins: [svelte()],
  optimizeDeps: { entries: ['tests/speech-theme-e2e/index.html'] },
  resolve: { alias: {
    '@tauri-apps/api/core': boundary,
    '@tauri-apps/api/event': boundary,
    '@tauri-apps/api/window': boundary
  } },
  server: { host: '127.0.0.1', port: 1447, strictPort: true }
});
