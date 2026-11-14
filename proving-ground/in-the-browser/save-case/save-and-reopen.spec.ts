import { expect, type Page } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { servedPaths } from '../test-rig/built-app';
import { holdStill } from '../test-rig/hold-still';
import { PolicyRecorder, test } from '../test-rig/policy-violations';
import { RequestLedger } from '../test-rig/request-ledger';

const FILMS = ['Front', 'Side', 'Bending left', 'Bending right'] as const;
const FOUND = /^\d+ vertebrae found\./;
const SEARCH_JOBS = ['film-pass', 'chain-pass', 'place-points', 'trained-pass', 'trained-side-pass'];

async function fourFilms(page: Page): Promise<void> {
  const stage = page.getByRole('main', { name: 'The films', exact: true });
  for (const name of FILMS) await expect(stage.getByRole('img', { name: `${name} film`, exact: true })).toBeVisible();
}

async function jobs(page: Page): Promise<string[]> {
  return page.evaluate(() => (window as unknown as { caseJourneyJobs: string[] }).caseJourneyJobs);
}

/** The real picker creates a detached file input, so catch it through the browser's chooser. */
async function chooseFile(page: Page, label: string, file: { name: string; mimeType: string; buffer: Buffer }): Promise<void> {
  const choosing = page.waitForEvent('filechooser');
  await page.getByRole('button', { name: label, exact: true }).click();
  await (await choosing).setFiles(file);
}

test('saves a local case under the content policy, reopens it, and builds again', async ({ baseURL, context, page }, testInfo) => {
  test.setTimeout(240_000);
  if (baseURL === undefined) throw new Error('playwright.config.ts sets baseURL');
  const ledger = await RequestLedger.attach(context, new URL(baseURL).origin);
  const policy = await PolicyRecorder.attach(context, new URL(baseURL).origin);
  const consoleFaults: string[] = [];
  const sockets: string[] = [];
  page.on('console', (message) => { if (message.type() === 'error') consoleFaults.push(message.text()); });
  page.on('websocket', (socket) => sockets.push(socket.url()));
  await context.addInitScript(() => {
    Object.defineProperty(window, 'showSaveFilePicker', { value: undefined, configurable: true });
    const calls: string[] = [];
    Object.defineProperty(window, 'caseJourneyJobs', { value: calls });
    const send = Reflect.get(Worker.prototype, 'postMessage');
    Worker.prototype.postMessage = function (this: Worker, input: unknown, transferOrOptions?: Transferable[] | StructuredSerializeOptions) {
      const message = input as { kind?: string; job?: string };
      if (message.kind === 'run' && message.job !== undefined) calls.push(message.job);
      Reflect.apply(send, this, [input, transferOrOptions]);
    };
  });
  await page.setViewportSize({ width: 1440, height: 900 });
  const response = await page.goto('/');
  expect(response?.headers()['content-security-policy']).toContain("connect-src 'self'");
  await expect(page.getByTestId('engine-status')).toHaveAttribute('data-status', 'ready');
  await expect(page.locator('html')).toHaveAttribute('data-ready', '');
  const dock = page.getByRole('contentinfo', { name: 'What to do now', exact: true });
  const sentence = dock.getByRole('heading', { level: 2 });
  const steps = page.getByRole('navigation', { name: 'Steps', exact: true });
  const toast = page.locator('.toast[data-shown] .toast-line');

  // The production build has only the made-up case; use the existing sample cycle if it grows.
  for (let sample = 0; sample < 7; sample += 1) {
    const before = await sentence.innerText();
    await dock.getByRole('button', { name: /sample/ }).click();
    await expect(sentence).not.toHaveText(before, { timeout: 90_000 });
    await expect(sentence).toHaveText(/^(All four films are in\.|The sample film is in\.|Sample film \d+ of \d+ is in\.)$/, { timeout: 90_000 });
    if (await sentence.innerText() === 'All four films are in.') break;
  }
  await fourFilms(page);
  await dock.getByRole('button', { name: 'Find the spine', exact: true }).click();
  await expect(sentence).toHaveText(FOUND, { timeout: 90_000 });
  const foundBefore = await sentence.innerText();
  const searchBefore = (await jobs(page)).filter((job) => SEARCH_JOBS.includes(job));
  expect(searchBefore.length).toBeGreaterThan(0);
  const save = page.getByRole('banner').getByRole('button', { name: 'Save case', exact: true });
  await expect(save).toBeVisible();
  const downloading = page.waitForEvent('download');
  await save.click();
  const download = await downloading;
  expect(download.suggestedFilename()).toBe('spine-case.spinecase');
  const savedPath = testInfo.outputPath('spine-case.spinecase');
  await download.saveAs(savedPath);
  expect(await download.failure()).toBeNull();
  const saved = await readFile(savedPath);
  expect(saved.length).toBeGreaterThan(0);
  expect([...saved.subarray(0, 4)]).toEqual([80, 75, 3, 4]);
  await expect(toast).toHaveText('Case sent to your downloads.');

  await steps.getByRole('button', { name: 'Films', exact: true }).click();
  await dock.getByRole('button', { name: 'Start over', exact: true }).click();
  await expect(sentence).toHaveText('Add the four films.');
  await expect(page.getByRole('main', { name: 'The films', exact: true }).getByRole('img')).toHaveCount(0);
  await expect(save).toHaveCount(0);
  await chooseFile(page, 'choose them', { name: download.suggestedFilename(), mimeType: 'application/zip', buffer: saved });
  await expect(toast).toHaveText('Opened a saved case.', { timeout: 90_000 });
  await expect(steps.getByRole('button', { name: 'Find', exact: true })).toHaveAttribute('aria-current', 'true');
  await expect(sentence).toHaveText(FOUND);
  const foundReopened = await sentence.innerText();
  // Reopened Side marks use their separate trained-source review; the vertebra count stays.
  expect(foundReopened.match(FOUND)?.[0]).toBe(foundBefore.match(FOUND)?.[0]);
  await fourFilms(page);
  expect((await jobs(page)).filter((job) => SEARCH_JOBS.includes(job))).toEqual(searchBefore);

  await dock.getByRole('button', { name: 'Build the 3D spine', exact: true }).click();
  await expect(sentence).toHaveText('The spine in 3D.', { timeout: 60_000 });
  const measurements = page.getByRole('complementary', { name: 'Measurements', exact: true });
  await expect(measurements.locator('.measures li').first()).toBeVisible({ timeout: 60_000 });
  const rows = await measurements.locator('.measures li').allTextContents();
  expect(rows.length).toBeGreaterThan(0);

  // A file's extension never decides its type: random bytes are refused by the film reader.
  await steps.getByRole('button', { name: 'Films', exact: true }).click();
  const beforeBad = await jobs(page);
  await chooseFile(page, 'Replace the Front film', { name: 'x.spinecase', mimeType: 'application/octet-stream', buffer: Buffer.from([1, 2, 3, 4, 5]) });
  await expect(dock.getByText('This file is not a film the app can read.', { exact: true })).toBeVisible();
  await fourFilms(page);
  await expect(save).toBeVisible();
  await steps.getByRole('button', { name: 'Find', exact: true }).click();
  await expect(sentence).toHaveText(foundReopened);
  await dock.getByRole('button', { name: 'Build the 3D spine', exact: true }).click();
  await expect(sentence).toHaveText('The spine in 3D.');
  await expect(measurements.locator('.measures li').first()).toBeVisible();
  expect(await measurements.locator('.measures li').allTextContents()).toEqual(rows);
  // The unchanged case reuses its fit and readings rather than recomputing either.
  expect((await jobs(page)).filter((job) => job === 'fit-spine' || job === 'measure-spine')).toEqual(beforeBad.filter((job) => job === 'fit-spine' || job === 'measure-spine'));

  await page.getByRole('button', { name: 'On this device only', exact: true }).click();
  await expect(page.locator('.note-count')).toHaveText('Requests sent: 0');
  await expect(page.locator('.note-blocked')).toHaveCount(0);
  await holdStill(async () => JSON.stringify([(await policy.violations(page)).length, ledger.entries.length, consoleFaults.length, sockets.length]));
  expect(await policy.violations(page)).toEqual([]);
  expect(consoleFaults).toEqual([]);
  expect(sockets).toEqual([]);
  expect(ledger.outside()).toEqual([]);
  const served = servedPaths();
  expect(ledger.sameOriginPaths().filter((path) => !served.has(path))).toEqual([]);
});
