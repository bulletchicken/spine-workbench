// Collects every content-policy violation reported in the page and in
// every worker it starts, each from before its own first line runs where
// the browser allows it.
//
// Each page and each worker gets a listener for `securitypolicyviolation`
// that reports through its console; Playwright passes a worker's console
// messages on to its page, even after the worker has closed.
// - The page's listener is an init script, so it runs before any of the
//   page's own scripts.
// - A worker's listener is put in front of the worker's script as the
//   script is served, so it runs before the worker's first line. This
//   needs the browser to hand the worker's script request to the route
//   as a script, as Chromium does; WebKit does not, so there a worker's
//   listener starts when Playwright reaches the worker, just after its
//   first lines. Every
//   script from the app's origin gets it, since a request does not say
//   whether a worker will run it; in the page it does nothing more. A
//   worker whose script starts with a static import is reported instead,
//   since its imports would run before the listener.
// - Every worker Playwright sees also gets the listener by evaluation, a
//   no-op when it is already there (and the only way in WebKit). A worker
//   it cannot reach is reported as a violation, so a missed worker fails
//   the check instead of passing, unless its script was served with the
//   listener in front, in which case the listener ran from its first
//   line anyway. So in Chromium a worker that closes before Playwright
//   reaches it is fine; in WebKit it is reported, and a journey that
//   retires a worker straight after starting it will fail there.

import type { BrowserContext, ConsoleMessage, Page, Route } from '@playwright/test';

export interface Violation {
  /** `page`, or the URL of the worker's script. */
  source: string;
  directive: string;
  blocked: string;
}

export interface RecorderOptions {
  /**
   * Code to run in every script from the app's origin straight after the
   * listener, before the script's own first line. Only the recorder's own
   * test uses it, to prove a violation on a worker's first line is seen.
   */
  workerPrelude?: string;
}

const MARK = 'content-policy-violation ';

/**
 * Run in the page and in each worker: report every violation through the
 * console. The page holds the policy twice (meta tag and header), so it
 * reports each of its own violations twice.
 */
function listen(mark: string): void {
  const scope = globalThis as unknown as { __listeningForPolicy?: true };
  if (scope.__listeningForPolicy) return;
  scope.__listeningForPolicy = true;
  addEventListener('securitypolicyviolation', (event) => {
    console.error(mark + JSON.stringify({ directive: event.effectiveDirective, blocked: event.blockedURI }));
  });
}

const LISTENER = `(${listen.toString()})(${JSON.stringify(MARK)});\n`;

export class PolicyRecorder {
  private readonly found: Violation[] = [];
  private readonly pending: Promise<void>[] = [];
  /** Every worker script URL Playwright has seen started. */
  private readonly workerScripts = new Set<string>();
  /** Scripts served that start with a static import. */
  private readonly importingScripts = new Set<string>();
  /** Scripts served with the listener in front. */
  private readonly listeningScripts = new Set<string>();

  private constructor(
    private readonly origin: string,
    private readonly options: RecorderOptions,
  ) {}

  /**
   * Starts recording in every page of `context` and in every worker those
   * pages start from `origin`, the app's own. Attach it after the request
   * ledger: the route added last runs first, and the ledger's ends the
   * chain.
   */
  static async attach(context: BrowserContext, origin: string, options: RecorderOptions = {}): Promise<PolicyRecorder> {
    const recorder = new PolicyRecorder(origin, options);
    await context.addInitScript(listen, MARK);
    await context.route('**', (route) => recorder.serve(route));
    const watch = (page: Page) => {
      page.on('console', (message) => {
        recorder.read(message);
      });
      page.on('worker', (worker) => {
        recorder.workerScripts.add(worker.url());
        recorder.pending.push(
          worker.evaluate(listen, MARK).catch(() => {
            if (recorder.listeningScripts.has(worker.url())) return;
            recorder.found.push({ source: worker.url(), directive: 'recorder-not-installed', blocked: '' });
          }),
        );
      });
    };
    context.pages().forEach(watch);
    context.on('page', watch);
    return recorder;
  }

  /** Every violation so far, from the page and from every worker, open or closed. */
  async violations(page: Page): Promise<Violation[]> {
    await Promise.all(this.pending);
    // A round trip to the page and each open worker: every console message
    // sent before the answer has arrived by the time the answer does.
    await Promise.all([
      page.evaluate(() => 0),
      ...page.workers().map((worker) => worker.evaluate(() => 0).catch(() => 0)),
    ]);
    const late = [...this.workerScripts]
      .filter((script) => this.importingScripts.has(script))
      .map((script) => ({ source: script, directive: 'worker-imports-before-listener', blocked: '' }));
    return [...this.found, ...late];
  }

  /** Serves a script from the app's origin with the listener in front; passes anything else on. */
  private async serve(route: Route): Promise<void> {
    const request = route.request();
    if (request.resourceType() !== 'script' || new URL(request.url()).origin !== this.origin) {
      await route.fallback();
      return;
    }
    const response = await route.fetch();
    const script = await response.text();
    if (/^\s*import\s*[\w{*"'`]/.test(script)) this.importingScripts.add(request.url());
    this.listeningScripts.add(request.url());
    await route.fulfill({ response, body: LISTENER + (this.options.workerPrelude ?? '') + script });
  }

  private read(message: ConsoleMessage): void {
    const text = message.text();
    if (!text.startsWith(MARK)) return;
    const { directive, blocked } = JSON.parse(text.slice(MARK.length)) as Omit<Violation, 'source'>;
    this.found.push({ source: message.worker()?.url() ?? 'page', directive, blocked });
  }
}
