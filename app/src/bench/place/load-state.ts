import { createStore } from 'zustand/vanilla';
import type { BenchBatch, BenchResult, BenchStop, LayoutInput } from '@/shapes';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { currentStep } from '@/view-state/current-step';
import { PLACE_LOAD_WORDS as W } from '@/words/place';
import { placeState } from './place-state';
export interface LoadFailure { kind: 'failed' | 'refused'; reason: string; missingLevels: boolean }
export interface LoadState { phase: 'idle' | 'running' | 'loaded' | 'failed' | 'stopped'; completed: number; total: number; result: BenchResult | null; batches: BenchBatch[]; stop: BenchStop; failure: LoadFailure | null; elapsedMs: number | null }
const initial: LoadState = { phase: 'idle', completed: 0, total: 0, result: null, batches: [], stop: 'rods-in', failure: null, elapsedMs: null };
export const loadState = createStore<LoadState>(() => initial);
let generation = 0;
export function loadFailure(error: unknown, input: LayoutInput): LoadFailure | null {
  if (error instanceof EngineFailure && error.error.code === 'pass-cancelled') return null;
  if (error instanceof EngineFailure && ['engine-crashed', 'engine-out-of-memory', 'engine-load-failed'].includes(error.error.code)) return { kind: 'failed', reason: W.failed, missingLevels: false };
  const levels = input.fit.vertebrae.map((v) => v.level), missingLevels = !Array.from({ length: 12 }, (_, i) => `T${String(i + 1)}`).concat(['L1','L2','L3','L4','L5']).every((level) => levels.includes(level as typeof levels[number]));
  return { kind: 'refused', reason: missingLevels ? W.missingLevels : W.refused, missingLevels };
}
export function backToPlacement(): void { generation++; loadState.setState(initial,true); }
export function stopLoading(): void {
  generation++;
  if (loadState.getState().phase === 'running') loadState.setState({ ...initial, phase: 'stopped' }, true);
  else if(loadState.getState().phase==='failed')loadState.setState(initial,true);
}
placeState.subscribe((state, before) => {
  if (state.key !== before.key || state.result !== before.result) { generation++; loadState.setState(initial, true); }
});
currentStep.subscribe((state) => { if (state.slug !== 'place') stopLoading(); });
/** A stopped or replaced slice cannot enter the retained batches or start another. */
export async function loadPlacement(engine: Pick<EngineClient, 'runLoadBench' | 'compareLayouts'>, input: LayoutInput): Promise<void> {
  if (loadState.getState().phase === 'running') return;
  const layout = placeState.getState().result?.layouts[0]; if (layout === undefined) return;
  const mine = ++generation, started = performance.now(), batches: BenchBatch[] = [], versions = input.versions;
  loadState.setState({ ...initial, phase: 'running', total: versions });
  placeState.setState({ review: null, sheet: false, pressed: null, hovered: null, keyboard:false });
  try {
    for (let from = 0; from < versions; from += 4) {
      const count = Math.min(4, versions - from);
      const batch = await engine.runLoadBench({ fit: input.fit, layouts: [layout], child: input.child, seed: input.seed, versions, versionFrom: from, versionCount: count });
      if (mine !== generation) return;
      batches.push(batch); loadState.setState({ completed: from + count });
    }
    const result = await engine.compareLayouts({ layouts: [layout], batches });
    if (mine !== generation) return;
    loadState.setState({ phase: 'loaded', result, batches, elapsedMs: performance.now() - started });
  } catch (error) {
    if (mine !== generation) return;
    const failure = loadFailure(error, input);
    loadState.setState({ ...initial, phase: failure === null ? 'idle' : 'failed', failure });
  }
}
