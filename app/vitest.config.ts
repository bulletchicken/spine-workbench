import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

// Tests do not load vite.config.ts: its build plugins run cargo, and no
// unit test needs the engine built.
const alias = { '@': fileURLToPath(new URL('./src', import.meta.url)) };

export default defineConfig({
  test: {
    projects: [
      {
        resolve: { alias },
        test: {
          name: 'logic',
          environment: 'node',
          include: ['src/**/*.test.ts', 'build-plugins/**/*.test.ts'],
        },
      },
      {
        resolve: { alias },
        test: {
          name: 'components',
          environment: 'jsdom',
          include: ['src/**/*.test.tsx'],
        },
      },
    ],
  },
});
