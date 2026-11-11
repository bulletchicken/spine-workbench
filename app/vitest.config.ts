import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

// Tests do not load vite.config.ts: its build plugins run cargo, and no
// unit test needs the engine built.
const alias = { '@': fileURLToPath(new URL('./src', import.meta.url)) };

// Property tests, and tests that paint whole films, take seconds on a busy
// or two-core machine. Under the default five seconds they fail for load,
// not for a fault.
const testTimeout = 30_000;

export default defineConfig({
  test: {
    projects: [
      {
        resolve: { alias },
        test: {
          name: 'logic',
          environment: 'node',
          testTimeout,
          include: ['src/**/*.test.ts', 'build-plugins/**/*.test.ts'],
        },
      },
      {
        resolve: { alias },
        test: {
          name: 'components',
          environment: 'jsdom',
          testTimeout,
          include: ['src/**/*.test.tsx'],
        },
      },
    ],
  },
});
