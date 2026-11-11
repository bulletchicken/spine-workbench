// The engine's case checks, run again whenever the case changes. Changes
// that come close together (a drag, a pass landing film by film) wait for
// a short quiet spell and are checked once. An answer for a version the
// case has already left is dropped, so the findings shown always describe
// the case they say they describe.

import type { CheckFinding, EngineError } from '@/shapes';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import type { CaseStoreApi } from './case-store';

/** How long the case must stay unchanged before it is checked. */
export const CHECK_DELAY_MS = 200;

export interface CaseChecks {
  /** The newest findings that describe a case the store held, or `null` before the first. */
  findings: readonly CheckFinding[] | null;
  /** The case version the findings describe. */
  checkedAt: number | null;
  /** Why the newest check failed, until one succeeds; `null` when it did not fail. */
  fault: EngineError | null;
}

export const NOT_CHECKED: CaseChecks = { findings: null, checkedAt: null, fault: null };

/**
 * Check the case in `store` with `engine` now (after the delay) and again
 * after every change, telling `onChecks` each result that still describes
 * the case. A check that a newer one replaced (`pass-cancelled`) is let
 * go by. Returns the way to stop.
 */
export function watchCaseChecks(store: CaseStoreApi, engine: EngineClient, onChecks: (checks: CaseChecks) => void, delayMs = CHECK_DELAY_MS): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let stopped = false;
  let latest = NOT_CHECKED;

  const stillCurrent = (version: number) => !stopped && store.getState().version === version;
  const publish = (checks: CaseChecks) => {
    latest = checks;
    onChecks(checks);
  };

  function check(): void {
    timer = null;
    const { case: c, version } = store.getState();
    engine.checkCase(c).then(
      (findings) => {
        if (stillCurrent(version)) publish({ findings, checkedAt: version, fault: null });
      },
      (thrown: unknown) => {
        if (!(thrown instanceof EngineFailure)) throw thrown;
        if (thrown.error.code === 'pass-cancelled' || !stillCurrent(version)) return;
        publish({ ...latest, fault: thrown.error });
      },
    );
  }

  function later(): void {
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(check, delayMs);
  }

  const unsubscribe = store.subscribe((state, previous) => {
    if (state.version !== previous.version) later();
  });
  later();

  return () => {
    stopped = true;
    unsubscribe();
    if (timer !== null) clearTimeout(timer);
  };
}
