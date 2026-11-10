import { defineConfig, devices } from '@playwright/test';

// Browser tests run only against the production build (`pnpm browser`
// builds it first), served by `vite preview`, which sends the content
// policy as a header on every file: dedicated workers obey the header,
// not the page's meta tag. Development mode has no policy.
//
// BROWSERS picks the engines, comma separated. Local runs default to
// Chromium; CI sets BROWSERS=chromium,webkit and installs WebKit there.

// Not Vite's default preview port, so a preview of some other project left
// running cannot collide with it. PREVIEW_PORT picks another, so two
// checkouts of this repository can run their browser tests at once.
const PORT = Number(process.env.PREVIEW_PORT ?? 4391);
if (!Number.isInteger(PORT) || PORT < 1024 || PORT > 65535) {
  throw new Error(`PREVIEW_PORT is not a usable port: ${String(process.env.PREVIEW_PORT)}`);
}

const ENGINES = {
  chromium: devices['Desktop Chrome'],
  webkit: devices['Desktop Safari'],
};

const chosen = (process.env.BROWSERS ?? 'chromium').split(',').map((name) => name.trim());
const projects = chosen.map((name) => {
  if (!(name in ENGINES)) throw new Error(`BROWSERS names an unknown browser: ${name}`);
  return { name, use: ENGINES[name as keyof typeof ENGINES] };
});

export default defineConfig({
  testDir: 'in-the-browser',
  testMatch: '**/*.spec.ts',
  outputDir: 'test-results',
  forbidOnly: process.env.CI !== undefined,
  reporter: process.env.CI === undefined ? 'list' : [['list'], ['github']],
  use: {
    baseURL: `http://localhost:${String(PORT)}`,
    serviceWorkers: 'block',
    trace: 'retain-on-failure',
  },
  projects,
  webServer: {
    command: `pnpm --dir ../app exec vite preview --port ${String(PORT)} --strictPort`,
    url: `http://localhost:${String(PORT)}`,
    // Never reuse a server already running: it might be the development
    // server, which has no content policy.
    reuseExistingServer: false,
  },
});
