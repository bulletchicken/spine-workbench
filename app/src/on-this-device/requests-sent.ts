// The counter behind "0 sent": what this page has asked of anywhere but
// its own origin, since it loaded. The badge, the popover and the status
// line all read it.
//
// Two things are counted, and both are told by where an address leads,
// never by what kind of call made it:
// - `sent`: every resource the page asked for from another origin, from
//   the browser's own record of what it loaded. The record is read from
//   its start, so nothing asked for before the app was up is missed.
// - `blocked`: every time the content policy refused an address outside
//   the app, from the event the browser raises when it does.
//
// A page can hold its policy twice, as a header and as a tag in its head,
// and then the browser reports one refusal twice, a moment apart. Reports
// of the same address under the same rule within `SAME_MOMENT_MS` are one
// attempt.
//
// It watches the page. A worker's own refusals are raised inside the
// worker and are not counted here; the policy still refuses them.

import { useSyncExternalStore } from 'react';

export interface RequestsSent {
  /** Requests that left for another origin. */
  sent: number;
  /** Attempts the browser refused before they left. */
  blocked: number;
}

/** Reports of one refusal arrive within a few milliseconds of each other; a person's next attempt does not. */
export const SAME_MOMENT_MS = 50;

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
  /** When it was reported, in milliseconds. */
  at: number;
}

/**
 * A tally of what left and what was refused. The app has one, fed by the
 * browser; tests make their own and feed it by hand.
 */
export function createTally(own: string) {
  let now: RequestsSent = { sent: 0, blocked: 0 };
  const lastRefused = new Map<string, number>();
  const listeners = new Set<() => void>();
  const set = (next: RequestsSent) => {
    now = next;
    for (const listener of listeners) listener();
  };
  return {
    read: (): RequestsSent => now,
    /** The browser loaded, or tried to load, something from `address`. */
    asked: (address: string): void => {
      if (leadsOutside(address, own)) set({ ...now, sent: now.sent + 1 });
    },
    /** The content policy refused something. */
    refused: ({ address, rule, at }: Refusal): void => {
      if (!leadsOutside(address, own)) return;
      const key = `${rule} ${address}`;
      const last = lastRefused.get(key);
      lastRefused.set(key, at);
      if (last !== undefined && at - last <= SAME_MOMENT_MS) return;
      set({ ...now, blocked: now.blocked + 1 });
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
    tally.refused({ address: event.blockedURI, rule: event.effectiveDirective, at: event.timeStamp });
  });
  counting = tally;
  return tally;
}

/** What this page has sent, and what the browser refused, since it loaded; it follows both as they change. */
export function useRequestsSent(): RequestsSent {
  const tally = pageTally();
  return useSyncExternalStore(tally.watch, tally.read);
}
