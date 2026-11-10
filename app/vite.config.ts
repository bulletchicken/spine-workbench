import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

// The app is a static site: one page, ES-module workers and the engine's
// own .wasm, all served from the same origin.
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  worker: { format: 'es' },
  build: {
    // Chrome and Safari 16.4 and later: the oldest browsers whose
    // WebAssembly has the 128-bit SIMD the engine is built with.
    target: ['chrome111', 'safari16.4'],
    // The preload polyfill fetches scripts itself, a fetch the lint rule
    // cannot see. Browsers without module preloading load them anyway.
    modulePreload: { polyfill: false },
  },
});
