import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import Terminal from 'vite-plugin-terminal'

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  clearScreen: false,
  plugins: [svelte(), Terminal({
      console: 'terminal', // Redirects console.log to terminal
      output: ['terminal', 'console'] // Optional: logs to both terminal and browser
    })],
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


