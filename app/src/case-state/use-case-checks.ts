// The case checks in a component. Every component that asks shares one
// watch per engine, started when the first one mounts and stopped when
// the last one goes, so the engine is asked once per change however many
// parts show the findings. While no part watches, nothing drops the
// findings when the case moves on, so they are shown again only if they
// describe the case as it is: one screen replacing another keeps them,
// and a part mounted after the case changed shows "not checked" until
// its own check lands.

import { useSyncExternalStore } from 'react';
import type { EngineClient } from '@/engine-room/engine-client';
import { caseStore } from './case-store';
import { NOT_CHECKED, watchCaseChecks, type CaseChecks } from './watch-case-checks';

interface SharedChecks {
  subscribe: (listener: () => void) => () => void;
  current: () => CaseChecks;
}

const shared = new WeakMap<EngineClient, SharedChecks>();

function sharedChecks(engine: EngineClient): SharedChecks {
  const known = shared.get(engine);
  if (known !== undefined) return known;
  const listeners = new Set<() => void>();
  let checks = NOT_CHECKED;
  let stop: (() => void) | null = null;
  const stillTrue = () => checks.checkedAt === caseStore.getState().version;
  const made: SharedChecks = {
    subscribe: (listener) => {
      listeners.add(listener);
      if (stop === null) {
        if (!stillTrue()) checks = NOT_CHECKED;
        stop = watchCaseChecks(caseStore, engine, (next) => {
          checks = next;
          for (const each of listeners) each();
        });
      }
      return () => {
        listeners.delete(listener);
        if (listeners.size === 0 && stop !== null) {
          stop();
          stop = null;
        }
      };
    },
    current: () => (stop !== null || stillTrue() ? checks : NOT_CHECKED),
  };
  shared.set(engine, made);
  return made;
}

/** The engine's findings about the open case, kept up with every change. */
export function useCaseChecks(engine: EngineClient): CaseChecks {
  const checks = sharedChecks(engine);
  return useSyncExternalStore(checks.subscribe, checks.current);
}
