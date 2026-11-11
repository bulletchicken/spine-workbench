// The tool in hand, whether the magnifier is on, and the pin being dragged.
// Never saved with the case. Picking another tool ends a drag in progress,
// since a drag belongs to the tool that started it.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';
import type { FilmKind, PinTarget } from '@/shapes';
import type { ToolId } from '@/toolbox/tool-contract';

/** The point being dragged, and the film it is on. */
export interface Dragging {
  film: FilmKind;
  target: PinTarget;
}

export interface ActiveToolState {
  /** The tool in hand, or `null` while none is picked up. */
  tool: ToolId | null;
  /** The magnifier is on. */
  magnifier: boolean;
  /**
   * A pin is being dragged, or `null`. While one is, the magnifier shows
   * even though `magnifier` stays as it was: read the magnifier as
   * `magnifier || dragging !== null`.
   */
  dragging: Dragging | null;
  /** Pick up a tool, or put every tool down with `null`. */
  pick(tool: ToolId | null): void;
  setMagnifier(on: boolean): void;
  startDrag(dragging: Dragging): void;
  endDrag(): void;
}

export type ActiveToolApi = StoreApi<ActiveToolState>;

/** A store with no tool in hand. The app has one, `activeTool`; tests make their own. */
export function createActiveTool(): ActiveToolApi {
  return createStore<ActiveToolState>()((set) => ({
    tool: null,
    magnifier: false,
    dragging: null,
    pick: (tool) => {
      set((state) => ({ tool, dragging: tool === state.tool ? state.dragging : null }));
    },
    setMagnifier: (magnifier) => {
      set({ magnifier });
    },
    startDrag: (dragging) => {
      set({ dragging });
    },
    endDrag: () => {
      set({ dragging: null });
    },
  }));
}

/** The app's active tool. */
export const activeTool: ActiveToolApi = createActiveTool();

/** The active tool, in a component. */
export function useActiveTool(): ActiveToolState {
  return useStore(activeTool);
}
