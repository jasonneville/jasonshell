import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  clearScreen: false,
  plugins: [svelte()],
  build: {
    rollupOptions: { input: resolve(__dirname, 'p03-experiment.html') },
  },
});
