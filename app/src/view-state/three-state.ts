import { currentStep } from './current-step';
import { createStore } from 'zustand/vanilla';
import type { FilmKind, Level } from '@/shapes';
export type ThreeView = 'front' | 'side' | 'top';
export function createThreeState() {
  return createStore<{ chosen: ThreeView | null; sheet: boolean; compared: FilmKind | null; pointed: Level | null; rowLevels: readonly Level[]; moving: boolean;
    choose(view: ThreeView | null): void; showSheet(on: boolean): void; compare(film: FilmKind | null): void; point(level: Level | null): void; light(levels: readonly Level[]): void; move(on: boolean): void; clear(): void;
  }>()((set) => ({
    chosen: null, sheet: false, compared: null, pointed: null, rowLevels: [], moving: false,
    choose: (chosen) => { set({ chosen }); }, showSheet: (sheet) => { set({ sheet, pointed: null, rowLevels: [] }); },
    compare: (compared) => { set({ compared, sheet: false, pointed: null, rowLevels: [], chosen: null }); },
    point: (pointed) => { set({ pointed }); }, light: (rowLevels) => { set({ rowLevels }); }, move: (moving) => { set({ moving }); },
    clear: () => { set({ chosen: null, sheet: false, compared: null, pointed: null, rowLevels: [], moving: false }); },
  }));
}
export const threeState = createThreeState();

currentStep.subscribe((state) => { if (state.slug !== 'three') threeState.getState().clear(); });
