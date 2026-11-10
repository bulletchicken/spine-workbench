// Records every request the page and its workers make, and stops any that
// would leave the app's own origin before it goes out.
//
// Two listeners feed one ledger: the context's route, which sees and can
// stop requests from the page and, in Chromium, from its workers; and the
// context's request event, which also reports requests a browser does not
// route. A request either one sees is recorded once.

import type { BrowserContext, Request } from '@playwright/test';

export interface LedgerEntry {
  url: URL;
  /** Playwright's resource type: document, script, fetch, image … */
  resourceType: string;
}

export class RequestLedger {
  readonly entries: LedgerEntry[] = [];
  private readonly seen = new Set<Request>();

  private constructor(readonly origin: string) {}

  /** Starts recording on `context`; anything not from `origin` is aborted. */
  static async attach(context: BrowserContext, origin: string): Promise<RequestLedger> {
    const ledger = new RequestLedger(origin);
    context.on('request', (request) => {
      ledger.record(request);
    });
    await context.route('**', async (route) => {
      const request = route.request();
      ledger.record(request);
      if (new URL(request.url()).origin === origin) await route.continue();
      else await route.abort('blockedbyclient');
    });
    return ledger;
  }

  /** Every request whose origin is not the app's. */
  outside(): string[] {
    return this.entries.filter(({ url }) => url.origin !== this.origin).map(({ url }) => url.href);
  }

  /** The paths of every request to the app's own origin. */
  sameOriginPaths(): string[] {
    return this.entries.filter(({ url }) => url.origin === this.origin).map(({ url }) => url.pathname);
  }

  /** The paths of every request of one resource type. */
  pathsOf(resourceType: string): string[] {
    return this.entries.filter((entry) => entry.resourceType === resourceType).map(({ url }) => url.pathname);
  }

  /** Whether any request went to `host`, by any route. */
  sawHost(host: string): boolean {
    return this.entries.some(({ url }) => url.host === host);
  }

  private record(request: Request): void {
    if (this.seen.has(request)) return;
    this.seen.add(request);
    this.entries.push({ url: new URL(request.url()), resourceType: request.resourceType() });
  }
}
