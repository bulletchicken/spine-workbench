// The browser's own guard: the content policy, on the production build.
//
// Checks the exact policy in the page's meta tag and in the header of
// every file, that eval is blocked in the page and in every worker, that
// the engine still compiles and answers, that a deliberate outside fetch
// is refused by the policy itself (not by the test's own request filter)
// in the page and in a worker, that a worker cannot be started from a
// blob: URL, and that no stand-in engine code is in the bundle.
//
// eval and fetch run inside a setTimeout callback: code Playwright
// evaluates directly is exempt from the page's policy, but a callback it
// schedules runs as the page's own code.

import { expect, type Page, type Worker } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { CONTENT_POLICY, CONTENT_POLICY_HEADER } from '../../../app/build-plugins/content-policy';
import { builtFiles, builtText } from '../test-rig/built-app';
import { PolicyRecorder, test } from '../test-rig/policy-violations';
import { RequestLedger } from '../test-rig/request-ledger';

const OUTSIDE = 'https://example.org/';

/** The workspace version in Cargo.toml, which the engine reports. */
function engineVersion(): string {
  const manifest = readFileSync(fileURLToPath(new URL('../../../Cargo.toml', import.meta.url)), 'utf8');
  const version = /\[workspace\.package\]\s+version = "([^"]+)"/.exec(manifest)?.[1];
  if (version === undefined) throw new Error('Cargo.toml has no [workspace.package] version');
  return version;
}

/** Opens the app and waits for the engine to answer from both workers. */
async function openApp(page: Page): Promise<readonly Worker[]> {
  await page.goto('/');
  await expect(page.getByTestId('engine-status')).toHaveAttribute('data-status', 'ready');
  expect(page.workers()).toHaveLength(2);
  return page.workers();
}

// Run in the page or in a worker, inside a scheduled callback.
const tryEval = () =>
  new Promise<string>((resolve) => {
    setTimeout(() => {
      try {
        (0, eval)('1');
        resolve('ran');
      } catch (error) {
        resolve((error as Error).name);
      }
    }, 0);
  });

const tryFunction = () =>
  new Promise<string>((resolve) => {
    setTimeout(() => {
      try {
        // The constructor through a typed name: what matters is the browser's answer.
        const FunctionFromText = Function as unknown as new (body: string) => () => unknown;
        new FunctionFromText('return 1')();
        resolve('ran');
      } catch (error) {
        resolve((error as Error).name);
      }
    }, 0);
  });

const tryFetch = (url: string) =>
  new Promise<string>((resolve) => {
    setTimeout(() => {
      fetch(url).then(
        () => {
          resolve('fetched');
        },
        () => {
          resolve('refused');
        },
      );
    }, 0);
  });

test('the policy is in the page, first in its head, and in every file’s header', async ({ page, request }) => {
  expect(CONTENT_POLICY_HEADER).toBe(`${CONTENT_POLICY}; frame-ancestors 'self'`);
  const response = await page.goto('/');
  expect(response?.headers()['content-security-policy']).toBe(CONTENT_POLICY_HEADER);
  const first = await page.evaluate(() => {
    const element = document.head.firstElementChild;
    return { tag: element?.tagName, equiv: element?.getAttribute('http-equiv'), content: element?.getAttribute('content') };
  });
  expect(first).toEqual({ tag: 'META', equiv: 'Content-Security-Policy', content: CONTENT_POLICY });

  for (const file of builtFiles()) {
    const served = await request.get(`/${file}`);
    expect(served.status(), file).toBe(200);
    expect(served.headers()['content-security-policy'], file).toBe(CONTENT_POLICY_HEADER);
  }
});

test('the engine compiles under the policy and answers from its workers', async ({ page }) => {
  await openApp(page);
  await expect(page.getByTestId('engine-version')).toHaveText(engineVersion());
});

test('eval and new Function are blocked in the page and in every worker', async ({ baseURL, browserName, context, page }) => {
  if (baseURL === undefined) throw new Error('playwright.config.ts sets baseURL');
  const policy = await PolicyRecorder.attach(context, new URL(baseURL).origin);
  const workers = await openApp(page);

  expect(await page.evaluate(tryEval)).toBe('EvalError');
  expect(await page.evaluate(tryFunction)).toBe('EvalError');
  for (const worker of workers) {
    expect(await worker.evaluate(tryEval)).toBe('EvalError');
    expect(await worker.evaluate(tryFunction)).toBe('EvalError');
  }
  // Every refusal above is asserted in both browsers. WebKit reports a
  // worker's blocked eval as an EvalError only, with no violation event,
  // so only Chromium is held to a report from each worker.
  const reporting = (await policy.violations(page)).filter((violation) => violation.directive === 'script-src');
  const reporters = browserName === 'chromium' ? ['page', ...workers.map((worker) => worker.url())] : ['page'];
  expect(new Set(reporting.map((violation) => violation.source))).toEqual(new Set(reporters));
});

test('an outside fetch is refused by the policy in the page and in a worker', async ({ baseURL, context, page }) => {
  if (baseURL === undefined) throw new Error('playwright.config.ts sets baseURL');
  const ledger = await RequestLedger.attach(context, new URL(baseURL).origin);
  const policy = await PolicyRecorder.attach(context, new URL(baseURL).origin);
  const workers = await openApp(page);

  expect(await page.evaluate(tryFetch, OUTSIDE)).toBe('refused');
  for (const worker of workers) expect(await worker.evaluate(tryFetch, OUTSIDE)).toBe('refused');

  // Refused before it went out: no route saw it, and the page and every
  // worker reported it.
  expect(ledger.sawHost(new URL(OUTSIDE).host)).toBe(false);
  const refusals = (await policy.violations(page)).filter(
    (violation) => violation.directive === 'connect-src' && violation.blocked === OUTSIDE,
  );
  expect(new Set(refusals.map((violation) => violation.source))).toEqual(
    new Set(['page', ...workers.map((worker) => worker.url())]),
  );
});

test('a worker cannot be started from a blob: URL', async ({ page }) => {
  await openApp(page);
  const outcome = await page.evaluate(
    () =>
      new Promise<string>((resolve) => {
        try {
          const url = URL.createObjectURL(new Blob(['postMessage("ran")'], { type: 'text/javascript' }));
          const worker = new Worker(url);
          worker.onmessage = () => {
            resolve('ran');
          };
          worker.onerror = () => {
            resolve('refused');
          };
        } catch {
          resolve('refused');
        }
      }),
  );
  expect(outcome).toBe('refused');
});

test('no stand-in engine code is in the bundle', () => {
  const carrying = builtFiles().filter((file) => /stand-in|standIn|StandIn/.test(builtText(file)));
  expect(carrying).toEqual([]);
});
