# app-shell

The frame around every step. It never rearranges: learn it once.

```
+--------------------+--------------------------------------------------+
| TopBar             |  StepRail        PromiseBadge  UndoRedo  Save    |   56
+--------------------+--------------------------------------------------+
| GuideColumn        |  the bench region                                |
|   the step's       |    bench/Bench.tsx            (found by path)    |
|   Screen           |    case-sheet/CaseSheet.tsx   (found by path)    |
+--------------------+--------------------------------------------------+
| StatusLine                                                            |   28
+-----------------------------------------------------------------------+
```

`AppFrame` mounts the bench once, for the whole visit, so films never
unmount between steps. The step on screen renders its guide into the
guide column and tells the bench what it wants.

| File | Holds |
|---|---|
| `AppFrame.tsx` | the three rows; `data-step`, `data-motion` and `data-tool` on the root |
| `TopBar.tsx` | the mark and name, the case, the rail, the promise, undo and redo, Save case |
| `StepRail.tsx`, `rail-steps.ts` | the numbered journey, and what each step shows, worked out from the case |
| `CaseTitle.tsx`, `case-saved.ts` | the case's name, and whether it is saved |
| `UndoRedo.tsx` | two buttons whose tooltips name the action |
| `StatusLine.tsx`, `status-readings.ts` | the standing claim; the pointer in millimetres; the pixel size; requests sent |
| `GuideColumn.tsx` | the left column: the step's `Screen`, inside the opening screen when there is one |
| `room-parts.ts` | finds the bench, the case sheet and the opening screen by path |
| `what-is-live.ts` | where undo and redo answer, and when Save case does |
| `frame-actions.ts`, `global-keys.ts` | undo, redo and save, by button and by Cmd or Ctrl+Z, Shift, and S |
| `use-top-bar.ts`, `use-engine-status.ts` | the stores and the engine, read for the frame |
| `step-attribute.ts` | the design's short `data-step` name for a slug that has one |
| `dev-scenes.ts` | development only: `?scene=<name>` |

## What a part mounted in the frame can rely on

- **The root** is the nearest ancestor with `data-step`, and it is always
  there, steps or none. The bench writes `data-lane` on it. `data-motion`
  is `reduced` when the system asks or view-state's preference does.
- **The bench region** is positioned and clips, so a dialog mounted in it
  veils the bench and nothing else. The frame draws the hairline down
  its left edge, over the region's first column of pixels: a part that
  fills the region and draws a line of its own in that place falls under
  it, and the line is never doubled. The rule under the top bar is the
  top bar's.
- **The lit button stands down** while view-state says the case sheet is
  open, so there is one ink button on the page.
- **Tab order** is the order of the page: top bar, guide, bench.

## The rail

A step shows a tick when its `canLeave` says everything it settles is
settled. Every step before the one on screen can be pressed: going back
is always allowed. A later step can be pressed only when it, and every
step from the one on screen up to it, is done, and it can be opened. The
rail never starts a step's work: that is the lit button's.

With more than four steps the done ones fold down to their ticks, and
under 1200 px only the step on screen keeps its name; a folded name is
in a tooltip, and what is spoken is always whole.

## Development

| Address | Shows |
|---|---|
| `?scene=<name>` | the app in a named state; a `*.scenes.ts` file anywhere under `src/` default-exports a list of `DevScene` |
| `?motion=reduced` | reduced motion for this visit |
| `?gallery=app-shell/AppFrame` | the frame on four fixture steps |
