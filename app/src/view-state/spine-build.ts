import { createStore } from 'zustand/vanilla';
import type { FitFromFilms, SpineFit } from '@/shapes';
import type { Bone } from '@/spine-3d/bones/read-bones';

export interface AcceptedSpine {
  key: string;
  input: FitFromFilms;
  fit: SpineFit;
  bones: readonly Bone[];
}
interface SpineBuildState {
  phase: 'idle' | 'building' | 'failed' | 'ready';
  busy: boolean;
  accepted: AcceptedSpine | null;
  begin(): void;
  markBusy(): void;
  accept(accepted: AcceptedSpine): void;
  fail(): void;
  cancel(): void;
  clear(): void;
}
export function createSpineBuild() {
  return createStore<SpineBuildState>()((set, get) => ({
    phase: 'idle', busy: false, accepted: null,
    begin: () => { set({ phase: 'building', busy: false, accepted: null }); },
    markBusy: () => { if (get().phase === 'building') set({ busy: true }); },
    accept: (accepted) => { set({ phase: 'ready', busy: false, accepted }); },
    fail: () => { set({ phase: 'failed', busy: false, accepted: null }); },
    cancel: () => { if (get().phase === 'building') set({ phase: 'idle', busy: false }); },
    clear: () => { set({ phase: 'idle', busy: false, accepted: null }); },
  }));
}
export const spineBuild = createSpineBuild();
