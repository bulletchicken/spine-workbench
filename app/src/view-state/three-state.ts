import { createStore } from 'zustand/vanilla';
export type ThreeView = 'front' | 'side' | 'top';
export function createThreeState() {
  return createStore<{ chosen: ThreeView | null; choose(view: ThreeView | null): void; clear(): void }>()((set) => ({
    chosen: null,
    choose: (chosen) => { set({ chosen }); },
    clear: () => { set({ chosen: null }); },
  }));
}
export const threeState = createThreeState();
