import type { BrowserContext, ConsoleMessage, Page, Route } from '@playwright/test';
import { describe, expect, it, vi } from 'vitest';
import { PolicyRecorder } from './policy-violations';

type Cleanup = (args: { context: BrowserContext }, use: () => Promise<void>) => Promise<void>;
const lifecycle = vi.hoisted(() => ({ cleanup: undefined as Cleanup | undefined }));
vi.mock('@playwright/test', () => ({ test: { extend: (fixtures: { policyRecorderCleanup: [Cleanup, { auto: true }] }) => {
  lifecycle.cleanup = fixtures.policyRecorderCleanup[0];
  return {};
} } }));

function rig() {
  let routeScript: ((route: Route) => Promise<void>) | undefined;
  let consoleMessage: ((message: ConsoleMessage) => void) | undefined;
  let pageClosed = false;
  let contextClosed = false;
  const page = {
    on: (event: string, listener: (message: ConsoleMessage) => void) => { if (event === 'console') consoleMessage = listener; },
    close: vi.fn(() => { pageClosed = true; return Promise.resolve(); }),
    isClosed: () => pageClosed,
    evaluate: () => Promise.resolve(0),
    workers: () => [],
  } as unknown as Page;
  const unrouteAll = vi.fn(() => Promise.resolve());
  const context = {
    addInitScript: vi.fn(() => Promise.resolve()),
    route: (_pattern: string, handler: (route: Route) => Promise<void>) => { routeScript = handler; return Promise.resolve(); },
    on: () => undefined,
    pages: () => [page],
    isClosed: () => contextClosed,
    unrouteAll,
  } as unknown as BrowserContext;
  const text = vi.fn(() => Promise.resolve('self.postMessage("ready");'));
  const response = { text };
  const fulfill = vi.fn<(options: { response: unknown; body: string }) => Promise<void>>(() => Promise.resolve());
  const route = {
    request: () => ({ resourceType: () => 'script', url: () => 'http://localhost/worker.js' }),
    fetch: () => Promise.resolve(response),
    fulfill,
  } as unknown as Route;
  return {
    context, page, route, text, fulfill, unrouteAll,
    serve: () => { if (routeScript === undefined) throw new Error('no route'); return routeScript(route); },
    closeContext: () => { contextClosed = true; },
    report: () => consoleMessage?.({ text: () => 'content-policy-violation {"directive":"connect-src","blocked":"https://example.org/"}', worker: () => null } as unknown as ConsoleMessage),
  };
}

describe('policy recorder requests', () => {
  it('keeps the listener before the worker prelude and script and still records a violation', async () => {
    const r = rig();
    const recorder = await PolicyRecorder.attach(r.context, 'http://localhost', { workerPrelude: 'firstLine();\n' });
    await r.serve();
    const body = r.fulfill.mock.calls[0]?.[0].body;
    expect(body).toContain('content-policy-violation ');
    expect(body).toMatch(/firstLine\(\);\nself\.postMessage\("ready"\);$/);
    r.report();
    expect(await recorder.violations(r.page)).toEqual([{ source: 'page', directive: 'connect-src', blocked: 'https://example.org/' }]);
  });

  it('rethrows an unexpected response-body error while the page and context are open', async () => {
    const r = rig();
    await PolicyRecorder.attach(r.context, 'http://localhost');
    const error = new Error('response body could not be decoded');
    r.text.mockRejectedValue(error);
    await expect(r.serve()).rejects.toBe(error);
    expect(r.fulfill).not.toHaveBeenCalled();
  });

  it.each(['apiResponse.text: Response has been disposed', 'route.fetch: Target page, context or browser has been closed'])('ends quietly for %s without fulfilling a script', async (message) => {
    const r = rig();
    await PolicyRecorder.attach(r.context, 'http://localhost');
    r.text.mockRejectedValue(new Error(message));
    await expect(r.serve()).resolves.toBeUndefined();
    expect(r.fulfill).not.toHaveBeenCalled();
  });

  it('closes pages then drains a late handler before the request client can be disposed', async () => {
    const r = rig();
    await PolicyRecorder.attach(r.context, 'http://localhost');
    let release: (() => void) | undefined;
    const reading = new Promise<string>((resolve) => { release = () => { resolve('self.postMessage("late");'); }; });
    r.text.mockReturnValue(reading);
    const serving = r.serve();
    await Promise.resolve();
    r.unrouteAll.mockImplementation(async () => {
      expect(r.page.isClosed()).toBe(true);
      expect(r.context.isClosed()).toBe(false);
      release?.();
      await serving;
    });
    if (lifecycle.cleanup === undefined) throw new Error('no cleanup fixture');
    await lifecycle.cleanup({ context: r.context }, () => Promise.resolve());
    expect(r.unrouteAll).toHaveBeenCalledExactlyOnceWith({ behavior: 'wait' });
    r.closeContext();
    expect(r.fulfill).toHaveBeenCalledTimes(1);
  });
});
