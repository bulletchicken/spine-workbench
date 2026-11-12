import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import { contentPolicy } from './build-plugins/content-policy.ts';
import { engineBuild } from './build-plugins/engine-build.ts';

// The app is a static site: one page, ES-module workers and the engine's
// own .wasm, all served from the same origin.
export default defineConfig(({ command, mode, isPreview }) => ({
  define: {
    'import.meta.env.SAMPLE_FILMS_ON_THIS_MACHINE': JSON.stringify(command === 'serve' && !isPreview || mode === 'films-on-this-machine'),
  },
  plugins: [engineBuild(), react(), contentPolicy()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  worker: { format: 'es' },
  // The bone shapes are a binary file that the 3D view imports as text
  // (`?inline`), so they arrive inside a script and are never fetched.
  assetsInclude: ['**/*.bin'],
  build: {
    outDir: mode === 'films-on-this-machine' ? 'films-on-this-machine/dist' : 'dist',
    // Chrome and Safari 16.4 and later: the oldest browsers whose
    // WebAssembly has the 128-bit SIMD the engine is built with.
    target: ['chrome111', 'safari16.4'],
    // The preload polyfill fetches scripts itself, a fetch the lint rule
    // cannot see. Browsers without module preloading load them anyway.
    modulePreload: { polyfill: false },
  },
}));
