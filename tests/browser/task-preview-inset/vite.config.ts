import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('.', import.meta.url));
const bridge = fileURLToPath(new URL('./bridge.ts', import.meta.url));
export default defineConfig({ root, plugins: [svelte()], resolve: { alias: Object.fromEntries(['core', 'event'].map(name => [`@tauri-apps/api/${name}`, bridge])) }, server: { host: '127.0.0.1', port: 4180, strictPort: true, fs: { allow: [fileURLToPath(new URL('../../../', import.meta.url))] } } });
