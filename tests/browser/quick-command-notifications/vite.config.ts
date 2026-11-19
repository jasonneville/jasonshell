import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const bridge = fileURLToPath(new URL('./bridge.ts', import.meta.url));
export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  plugins: [svelte()],
  resolve: { alias: Object.fromEntries(['core', 'event', 'window', 'webviewWindow'].map(name => [`@tauri-apps/api/${name}`, bridge])) },
  server: { host: '127.0.0.1', port: 4196, strictPort: true, fs: { allow: [fileURLToPath(new URL('../../../', import.meta.url))] } },
});
