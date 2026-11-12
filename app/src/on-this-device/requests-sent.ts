// The counter behind "0 sent": what this page has sent to anywhere but its
// own origin, since it loaded. The badge, the popover and the status line
// all read it.
//
// Two things are counted, and both are told by where an address leads,
// never by what kind of call made it:
// - `blocked`: attempts the content policy refused outside the app,
//   from the events the browser raises when it does.
// - `sent`: every resource the page asked for from another origin and the
//   policy did not refuse, from the browser's own record of what it asked
//   for. The record is read from its start, so nothing asked for before
//   the app was up is missed. The browser lists a refused request in that
//   record too, as if it had failed on the way, so an address the policy
//   refused is taken out: it was asked for, and it never left.
//
// A page can hold its policy twice, as a header and as a tag in its head,
// and then the browser reports one refusal twice. Each event carries the
// policy's text in originalPolicy. Count outside reports for each text
// and take the largest count: each policy reports each attempt once, so
// the count is exact with one policy or with our two distinct policies,
// whatever the order or timing of their reports. If a host repeats the
// tag's exact text in its header, both copies go into one count instead:
// that is an upper bound, which can count one attempt twice.
//
// It watches the page. A worker's own refusals are raised inside the
// worker and are not counted here; the policy still refuses them.

import { useSyncExternalStore } from 'react';

export interface RequestsSent {
  /** Requests for another origin that the policy did not refuse. */
  sent: number;
  /** Attempts refused outside the app; an upper bound if policy copies have identical text. */
  blocked: number;
}

/** True when `address` leads somewhere other than `own`, the page's origin. An address that leads nowhere (eval, inline, data) is not outside. */
export function leadsOutside(address: string, own: string): boolean {
  let origin: string;
  try {
    origin = new URL(address).origin;
  } catch {
    return false;
  }
  return origin !== 'null' && origin !== own;
}

/** One refusal, as the browser reports it. */
export interface Refusal {
  /** The address that was refused, or a word such as `eval` for a refusal that is not about an address. */
  address: string;
  /** The rule of the policy that refused it. */
  rule: string;
  /** The text of the policy that reported it, from the event's originalPolicy. */
  policy: string;
}

/** An address without its fragment, which the browser's record keeps and its refusals drop. */
function withoutFragment(address: string): string {
  const url = new URL(address);
  url.hash = '';
  return url.href;
}

/**
 * A tally of what left and what was refused. The app has one, fed by the
 * browser; tests make their own and feed it by hand. The browser may tell
 * of a refusal before or after it records the request, so the count of
 * what was sent is worked out afresh from both each time either changes.
 */
export function createTally(own: string) {
  /** Every address outside the app the page asked for. */
  const askedFor: string[] = [];
  /** Every address outside the app the policy refused, and that address's origin when the browser gave no more. */
  const refusedAddresses = new Set<string>();
  const refusalsByPolicy = new Map<string, number>();
  let blocked = 0;
  let now: RequestsSent = { sent: 0, blocked: 0 };
  const listeners = new Set<() => void>();

  const wasRefused = (address: string) => refusedAddresses.has(address) || refusedAddresses.has(new URL(address).origin);
  const settle = () => {
    const sent = askedFor.filter((address) => !wasRefused(address)).length;
    if (sent === now.sent && blocked === now.blocked) return;
    now = { sent, blocked };
    for (const listener of listeners) listener();
  };

  return {
    read: (): RequestsSent => now,
    /** The browser's record lists a request for `address`. */
    asked: (address: string): void => {
      if (!leadsOutside(address, own)) return;
      askedFor.push(withoutFragment(address));
      settle();
    },
    /** The content policy refused something. */
    refused: ({ address, policy }: Refusal): void => {
      if (!leadsOutside(address, own)) return;
      refusedAddresses.add(withoutFragment(address));
      // A refusal that gives an origin and nothing more stands for every address there.
      const { origin, pathname, search } = new URL(address);
      if (pathname === '/' && search === '') refusedAddresses.add(origin);
      const count = (refusalsByPolicy.get(policy) ?? 0) + 1;
      refusalsByPolicy.set(policy, count);
      blocked = Math.max(blocked, count);
      settle();
    },
    watch: (listener: () => void): (() => void) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

type Tally = ReturnType<typeof createTally>;
let counting: Tally | null = null;

/** The page's own tally. Counting starts the first time it is asked for: the browser's record of what it loaded, from its start, and every refusal from then on. */
function pageTally(): Tally {
  if (counting !== null) return counting;
  const tally = createTally(window.location.origin);
  new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) tally.asked(entry.name);
  }).observe({ type: 'resource', buffered: true });
  document.addEventListener('securitypolicyviolation', (event) => {
    tally.refused({ address: event.blockedURI, rule: event.effectiveDirective, policy: event.originalPolicy });
  });
  counting = tally;
  return tally;
}

/** What this page has sent, and what the browser refused, since it loaded; it follows both as they change. */
export function useRequestsSent(): RequestsSent {
  const tally = pageTally();
  return useSyncExternalStore(tally.watch, tally.read);
}
