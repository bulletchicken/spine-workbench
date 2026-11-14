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
// 5. goes offline as soon as the engine is ready, then makes the sample
//    case, finds its spine, reviews it, revisits both moments at 640 wide
//    and starts over. In WebKit it goes offline once the four films are
//    in: each film starts a worker of its own, and WebKit will not load a
//    worker's script offline unless the server lets it keep a copy, which
//    `vite preview` does not;
// 6. at the end of the journey, waits until the violations, requests and
//    sockets hold still for a second (test-rig/hold-still.ts) before it
//    reads them, so a call made on a timer within about a second of the
//    last step is still caught; one on a longer timer may not be. This
//    stays the last step however long the journey grows.

import { expect, type Locator, type Page } from '@playwright/test';
import { servedPaths } from '../test-rig/built-app';
import { holdStill } from '../test-rig/hold-still';
import { PolicyRecorder, test } from '../test-rig/policy-violations';
import { RequestLedger } from '../test-rig/request-ledger';

const FILMS = ['Front', 'Side', 'Bending left', 'Bending right'] as const;
const FOUND = /^\d+ vertebrae found\./;
const REVIEWED = /^\d+ vertebrae found\.$/;
type Box = { x: number; y: number; width: number; height: number };

async function boxOf(part: Locator): Promise<Box> {
  const box = await part.boundingBox();
  if (box === null) throw new Error(`no visible box for ${part.toString()}`);
  return box;
}

function meet(a: Box, b: Box): boolean {
  return a.x + a.width > b.x && b.x + b.width > a.x && a.y + a.height > b.y && b.y + b.height > a.y;
}

/**
 * The labelled canvas fills its tile; the caption is found by its words.
 * Films are counted by name: once outlines are drawn, each tile's overlay
 * is a picture too.
 */
async function fourFilms(page: Page): Promise<void> {
  const stage = page.getByRole('main', { name: 'The films', exact: true });
  await expect(stage.getByRole('img', { name: / film$/ })).toHaveCount(4);
  for (const name of FILMS) await expect(stage.getByRole('img', { name: `${name} film`, exact: true })).toBeVisible();
}

async function settledFilms(page: Page): Promise<void> {
  await expect(page.locator('[data-flying]')).toHaveCount(0);
  const stage = page.getByRole('main', { name: 'The films', exact: true });
  await expect.poll(() => stage.evaluate((part) => part.getAnimations({ subtree: true })
    .filter((animation) => animation.pending || animation.playState === 'running').length), { timeout: 10_000 }).toBe(0);
}

/** Read fresh boxes on every poll: fonts, resize and scene travel must have settled. */
async function layoutFaults(page: Page): Promise<string[]> {
  const faults: string[] = [];
  const banner = page.getByRole('banner');
  // Include the mark beside the product name, which shares its parent in the bar.
  const brand = banner.getByText('Spine Workbench', { exact: true }).locator('..');
  const steps = page.getByRole('navigation', { name: 'Steps', exact: true });
  const badge = banner.getByRole('button', { name: 'On this device only', exact: true });
  const parts = [brand, steps, badge];
  const boxes = await Promise.all(parts.map(boxOf));
  for (const [at, a] of boxes.entries()) {
    for (const b of boxes.slice(at + 1)) if (meet(a, b)) faults.push('bar parts meet');
  }

  const stage = page.getByRole('main', { name: 'The films', exact: true });
  const room = await boxOf(stage);
  for (const name of FILMS) {
    for (const part of [stage.getByRole('img', { name: `${name} film`, exact: true }), stage.getByText(name, { exact: true })]) {
      const box = await boxOf(part);
      // Browser layout and transforms can round the same edge differently by a fraction of a pixel.
      if (box.x < room.x - 0.5 || box.y < room.y - 0.5 || box.x + box.width > room.x + room.width + 0.5 || box.y + box.height > room.y + room.height + 0.5) {
        faults.push(`${name} or its caption leaves the stage`);
      }
    }
  }

  const dock = page.getByRole('contentinfo', { name: 'What to do now', exact: true });
  const sentence = dock.getByRole('heading', { level: 2 });
  if (await sentence.evaluate((part) => part.scrollWidth !== part.clientWidth)) faults.push('dock sentence is cut');
  const words = await boxOf(sentence);
  for (const button of await dock.getByRole('button').all()) {
    if (meet(words, await boxOf(button))) faults.push('dock sentence meets a button');
  }
  const noticeLines = await page.getByTestId('engine-status').evaluate((part) => {
    const text = part.firstChild;
    if (text === null || text.nodeType !== Node.TEXT_NODE) throw new Error('the notice has no visible words');
    const range = document.createRange();
    range.selectNodeContents(text);
    return range.getClientRects().length;
  });
  if (noticeLines !== 1) faults.push('notice wraps');
  return faults;
}

test('nothing leaves the machine', async ({ baseURL, browserName, context, page }) => {
  // Drawing four films, the real search and a review each have their own wait below.
  test.setTimeout(240_000);
  if (baseURL === undefined) throw new Error('playwright.config.ts sets baseURL');
  const ledger = await RequestLedger.attach(context, new URL(baseURL).origin);
  const policy = await PolicyRecorder.attach(context, new URL(baseURL).origin);
  const sockets: string[] = [];
  page.on('websocket', (socket) => sockets.push(socket.url()));

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  const status = page.getByTestId('engine-status');
  await expect(status).toHaveAttribute('data-status', 'ready');
  // See 5 above: WebKit must still be able to load the film workers' script.
  const offlineFromTheStart = browserName !== 'webkit';
  if (offlineFromTheStart) await context.setOffline(true);

  // Check the initial pair before opening films starts one worker per film kind.
  await expect(status).toHaveAttribute('data-status', 'ready');
  await expect(page.getByTestId('engine-version')).toHaveText(/^\d+\.\d+\.\d+$/);
  expect(page.workers()).toHaveLength(2);

  const dock = page.getByRole('contentinfo', { name: 'What to do now', exact: true });
  const sentence = dock.getByRole('heading', { level: 2 });
  const steps = page.getByRole('navigation', { name: 'Steps', exact: true });
  await expect(sentence).toHaveText('Add the four films.');
  await expect(steps.getByRole('button', { name: 'Films', exact: true })).toHaveAttribute('aria-current', 'true');
  await expect(dock.getByRole('button', { name: 'Find the spine', exact: true })).toBeDisabled();
  for (const name of FILMS) await expect(page.getByRole('button', { name: `Choose the ${name} film`, exact: true })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('data-ready', '');

  // The same control cycles through up to six local samples before the made-up case.
  for (let sample = 0; sample < 7; sample += 1) {
    const before = await sentence.innerText();
    await dock.getByRole('button', { name: /sample/ }).click();
    await expect(sentence).not.toHaveText(before, { timeout: 90_000 });
    await expect(sentence).toHaveText(/^(All four films are in\.|The sample film is in\.|Sample film \d+ of \d+ is in\.)$/, { timeout: 90_000 });
    if (await sentence.innerText() === 'All four films are in.') break;
  }
  await expect(sentence).toHaveText('All four films are in.');
  await fourFilms(page);
  if (!offlineFromTheStart) await context.setOffline(true);
  await dock.getByRole('button', { name: 'Find the spine', exact: true }).click();
  await expect(steps.getByRole('button', { name: 'Find', exact: true })).toHaveAttribute('aria-current', 'true');
  await expect(sentence).toHaveText(FOUND, { timeout: 90_000 });
  for (const name of FILMS) await expect(page.getByRole('group', { name: `Outlines on the ${name} film`, exact: true })).toBeVisible();

  const review = dock.getByRole('button', { name: /^Review/ });
  if (await review.count() > 0) {
    await review.click();
    await expect(sentence).toHaveText(/needs a look\.$/);
    // Each press holds for 560 ms. Wait for new words before pressing again.
    for (let looked = 0; looked < 18 && await dock.getByRole('button', { name: 'Looks right', exact: true }).count() > 0; looked += 1) {
      const before = await sentence.innerText();
      await expect(sentence).toHaveText(/needs a look\.$/);
      await dock.getByRole('button', { name: 'Looks right', exact: true }).click();
      await expect(sentence).not.toHaveText(before, { timeout: 10_000 });
    }
  }
  await expect(sentence).toHaveText(REVIEWED);
  await expect(dock.getByText('All of them look right.', { exact: true })).toBeVisible();
  // The fit runs on this machine too: the reviewed search builds the 3D spine and opens it.
  await dock.getByRole('button', { name: 'Build the 3D spine', exact: true }).click();
  await expect(sentence).toHaveText('The spine in 3D.', { timeout: 60_000 });
  await expect(steps.getByRole('button', { name: '3D', exact: true })).toBeEnabled();

  await steps.getByRole('button', { name: 'Films', exact: true }).click();
  await expect(sentence).toHaveText('All four films are in.');
  await fourFilms(page);
  await page.setViewportSize({ width: 640, height: 800 });
  await settledFilms(page);
  await expect.poll(() => layoutFaults(page), { timeout: 10_000 }).toEqual([]);
  // Returning through the step control reuses the finished search, and the looks made in it.
  await steps.getByRole('button', { name: 'Find', exact: true }).click();
  await expect(sentence).toHaveText(REVIEWED);
  await fourFilms(page);
  await settledFilms(page);
  await expect.poll(() => layoutFaults(page), { timeout: 10_000 }).toEqual([]);
  await steps.getByRole('button', { name: 'Films', exact: true }).click();
  await expect(sentence).toHaveText('All four films are in.');
  await dock.getByRole('button', { name: 'Start over', exact: true }).click();
  await expect(sentence).toHaveText('Add the four films.');
  await expect(page.getByRole('main', { name: 'The films', exact: true }).getByRole('img')).toHaveCount(0);

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
