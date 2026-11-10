// The policy recorder that nothing-leaves.spec.ts relies on must see a
// violation inside a worker, not only in the page, and must see it even
// on the worker's very first line, before Playwright could reach the
// worker any other way. Here each of the app's workers tries to reach
// outside, by fetch and by WebSocket, as its first statement; the
// recorder must report both attempts from each worker. If this fails, the no-network spec
// would pass a worker that tries to leak.
//
// Chromium only: WebKit does not hand a worker's script request to the
// route as a script, so the recorder cannot put its listener in front of
// a worker's first line there (see test-rig/policy-violations.ts). Later violations in a WebKit
// worker are still recorded, as content-policy.spec.ts shows.

import { expect, test } from '@playwright/test';
import { PolicyRecorder } from '../test-rig/policy-violations';

const OUTSIDE = 'https://example.org/';
const OUTSIDE_SOCKET = 'wss://example.org/';

const FIRST_LINE = `fetch('${OUTSIDE}').catch(() => {}); try { new WebSocket('${OUTSIDE_SOCKET}'); } catch {}\n`;

test('a worker reaching outside on its first line is reported', async ({ baseURL, browserName, context, page }) => {
  test.skip(browserName !== 'chromium', 'WebKit does not hand a worker script request to the route');
  if (baseURL === undefined) throw new Error('playwright.config.ts sets baseURL');
  const policy = await PolicyRecorder.attach(context, new URL(baseURL).origin, { workerPrelude: FIRST_LINE });
  await page.goto('/');
  await expect(page.getByTestId('engine-status')).toHaveAttribute('data-status', 'ready');
  expect(page.workers()).toHaveLength(2);

  const fromWorkers = async () =>
    (await policy.violations(page))
      .filter((violation) => violation.source !== 'page' && violation.directive === 'connect-src')
      .map((violation) => `${violation.source} ${violation.blocked}`)
      .sort();
  const expected = page
    .workers()
    .flatMap((worker) => [`${worker.url()} ${OUTSIDE}`, `${worker.url()} ${OUTSIDE_SOCKET}`])
    .sort();
  await expect.poll(fromWorkers).toEqual(expected);
});
