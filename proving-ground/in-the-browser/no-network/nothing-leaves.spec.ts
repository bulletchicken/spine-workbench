// The test that fails if any request leaves the machine.
//
// It runs the production build, served by `vite preview` with the content
// policy header, and:
// 1. records every request from the page and its workers, aborting any
//    that is not to the app's own origin;
// 2. fails on any WebSocket;
// 3. fails on any content-policy violation in the page or in any of its
//    workers, recorded from before each one's first line runs (in
//    WebKit, a worker's from when Playwright reaches it);
// 4. requires every same-origin request to be for a file in the build,
//    and exactly one fetch: the engine's own hashed .wasm;
// 5. goes offline as soon as the engine is ready, then carries on. Until
//    the screens exist the journey is the bare shell: the engine answering
//    from its workers;
// 6. at the end of the journey, waits until the violations, requests and
//    sockets hold still for a second (test-rig/hold-still.ts) before it
//    reads them, so a call made on a timer within about a second of the
//    last step is still caught; one on a longer timer may not be. This
//    stays the last step however long the journey grows.

import { expect, test } from '@playwright/test';
import { servedPaths } from '../test-rig/built-app';
import { holdStill } from '../test-rig/hold-still';
import { PolicyRecorder } from '../test-rig/policy-violations';
import { RequestLedger } from '../test-rig/request-ledger';

test('nothing leaves the machine', async ({ baseURL, context, page }) => {
  if (baseURL === undefined) throw new Error('playwright.config.ts sets baseURL');
  const ledger = await RequestLedger.attach(context, new URL(baseURL).origin);
  const policy = await PolicyRecorder.attach(context, new URL(baseURL).origin);
  const sockets: string[] = [];
  page.on('websocket', (socket) => sockets.push(socket.url()));

  await page.goto('/');
  const status = page.getByTestId('engine-status');
  await expect(status).toHaveAttribute('data-status', 'ready');
  await context.setOffline(true);

  // The journey, offline. Once the screens exist it is the whole
  // made-up-spine journey, save and reopen included.
  await expect(status).toHaveAttribute('data-status', 'ready');
  await expect(page.getByTestId('engine-version')).toHaveText(/^\d+\.\d+\.\d+$/);
  expect(page.workers()).toHaveLength(2);

  await holdStill(async () =>
    JSON.stringify([(await policy.violations(page)).length, ledger.entries.length, sockets.length]),
  );
  expect(await policy.violations(page)).toEqual([]);
  expect(sockets).toEqual([]);
  expect(ledger.outside()).toEqual([]);
  const served = servedPaths();
  expect(ledger.sameOriginPaths().filter((path) => !served.has(path))).toEqual([]);
  const fetches = ledger.pathsOf('fetch');
  expect(fetches).toHaveLength(1);
  expect(fetches[0]).toMatch(/^\/assets\/wasm_bridge_bg-[\w-]+\.wasm$/);
});
