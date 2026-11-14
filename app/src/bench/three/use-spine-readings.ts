import { useMemo, useSyncExternalStore } from 'react';
import { useStore } from 'zustand';
import { useCase } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { useEngine } from '@/engine-room/use-engine';
import type { EngineClient } from '@/engine-room/engine-client';
import { fitSources } from '@/view-state/fit-sources';
import { spineBuild, type AcceptedSpine } from '@/view-state/spine-build';
import { filmToFitFor } from './fit-input';
import { readingsView } from './readings-view';
type State = { phase: 'loading' } | { phase: 'failed' } | { phase: 'ready'; view: ReturnType<typeof readingsView> };
interface Entry { key: string; state: State; listeners: Set<() => void>; engine: EngineClient; retry: () => void }
let cached: Entry | null = null;
spineBuild.subscribe((state) => { if (state.accepted === null) cached = null; });
export function useSpineReadings(accepted: AcceptedSpine) {
  const engine = useEngine();
  const c = useCase((s) => s.case);
  const sources = useStore(fitSources, (s) => s.films);
  const openings = useStore(filmPixels, (s) => s.openings);
  const input = useMemo(() => ({ fit: accepted.fit, front: accepted.input.front, side: accepted.fit.sideFilm === null ? null : accepted.input.side,
    bendLeft: filmToFitFor(c, 'bend-left', sources, openings), bendRight: filmToFitFor(c, 'bend-right', sources, openings), versions: 32, seed: 7 }), [accepted, c, sources, openings]);
  const key = JSON.stringify({ input, openings });
  const entry = useMemo(() => entryFor(key, engine, input, accepted), [key, engine, input, accepted]);
  const state = useSyncExternalStore((listen) => { entry.listeners.add(listen); if (entry.state.phase === 'loading' && entry.listeners.size === 1 && !started.has(entry)) { started.add(entry); entry.retry(); } return () => { entry.listeners.delete(listen); }; }, () => entry.state);
  return { ...state, retry: entry.retry };
}
const started = new WeakSet<Entry>();

function entryFor(key: string, engine: EngineClient, input: import('@/shapes').ReadingsInput, accepted: AcceptedSpine): Entry {
    if (cached?.key === key && cached.engine === engine) return cached;
    let attempt = 0;
    const made: Entry = { key, engine, state: { phase: 'loading' }, listeners: new Set(), retry: () => {
      const mine = ++attempt;
      made.state = { phase: 'loading' };
      for (const listen of made.listeners) listen();
      void engine.measureSpine(input).then((result) => {
        if (cached !== made || mine !== attempt) return;
        try { made.state = { phase: 'ready', view: readingsView(result, accepted.input.front) }; }
        catch { made.state = { phase: 'failed' }; }
        for (const listen of made.listeners) listen();
      }, () => { if (cached !== made || mine !== attempt) return; made.state = { phase: 'failed' }; for (const listen of made.listeners) listen(); });
    } };
    cached = made;
    return made;
}
