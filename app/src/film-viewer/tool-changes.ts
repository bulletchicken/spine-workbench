// How a running tool tells the viewer it has something new to show. The
// frozen contract hands `changed` to the tool when it is started, by
// whoever starts it; that code passes this on:
//
//   let tool: ActiveTool | null = null;
//   tool = contract.start(session, () => {
//     if (tool !== null) toolChanged(tool);
//   });
//
// and the viewer showing that tool reads its `view()` again. A change
// told while `start` is still running has no tool to name yet and is let
// go: no viewer is showing the tool then, and the first one to show it
// reads its `view()` anyway.

import type { ActiveTool } from '@/toolbox/tool-contract';

const watchers = new WeakMap<ActiveTool, Set<() => void>>();

/** The tool's view has changed: every viewer showing it reads `view()` again. */
export function toolChanged(tool: ActiveTool): void {
  for (const watcher of watchers.get(tool) ?? []) watcher();
}

/** Be told when `tool` changes; returns the way to stop. */
export function watchTool(tool: ActiveTool, watcher: () => void): () => void {
  const set = watchers.get(tool) ?? new Set();
  set.add(watcher);
  watchers.set(tool, set);
  return () => {
    set.delete(watcher);
  };
}
