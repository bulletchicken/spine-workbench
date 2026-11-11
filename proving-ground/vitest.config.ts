import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

const alias = { '@': fileURLToPath(new URL('../app/src', import.meta.url)) };

// Two projects, because they need different things built first:
// - parity runs the release engine in Node (`pnpm test` builds it first);
// - build scans the production build in app/dist (`pnpm browser` builds
//   it first, then runs this project before Playwright).
export default defineConfig({
  test: {
    projects: [
      { resolve: { alias }, test: { name: 'parity', environment: 'node', include: ['parity/**/*.test.ts'] } },
      { test: { name: 'build', environment: 'node', include: ['in-the-browser/**/*.test.ts'] } },
    ],
  },
});
