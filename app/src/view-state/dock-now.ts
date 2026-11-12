// What the current step hands to the frame's one dock. Never saved with the case.
import { useEffect } from 'react';
import { useStore } from 'zustand';
import { createStore } from 'zustand/vanilla';
import type { DockSpec } from '@/guide/dock-spec';

interface DockNowState {
  spec: DockSpec | null;
  show(spec: DockSpec | null): void;
}

export const dockNow = createStore<DockNowState>()((set) => ({
  spec: null,
  show: (spec) => { set({ spec }); },
}));

export function useDockNow(): DockSpec | null {
  return useStore(dockNow, (state) => state.spec);
}

/** The step draws no dock of its own, and clears its words when it leaves. */
export function useShowInDock(spec: DockSpec): void {
  useEffect(() => {
    dockNow.getState().show(spec);
    return () => { dockNow.getState().show(null); };
  }, [spec]);
}
