# app-shell

The one stage, with a capsule bar above it and a dock and notice below.
The same bench stays mounted as the step changes. All three regions sit
inside the centred frame; extra window space becomes margin.

```
app-frame
  glow
  frame
    bar: brand, steps, badge
    note: the promise and its count
    stage: Bench, CaseSheet
    the step's Screen, inside OpeningCase when present
    dock: the current sentence and actions
    notice
```

| File | Holds |
|---|---|
| `AppFrame.tsx` | the frame, its persistent stage and shared dock; `data-step`, `data-motion` and `data-tool` on the root, `data-moment` on the page |
| `TopBar.tsx`, `StepSwitch.tsx` | the name, the step control and the badge; the note is a sibling of the bar |
| `PreviewNotice.tsx` | the approved notice and the engine status and version hooks used by browser tests |
| `rail-steps.ts` | the current step and which steps may be opened, worked out from the case |
| `room-parts.ts` | finds the bench, the case sheet and the opening screen by path |
| `use-top-bar.ts`, `use-engine-status.ts` | the stores and engine state read by the frame |
| `step-attribute.ts` | the short `data-step` name for a slug that has one |
| `dev-scenes.ts` | development only: `?scene=<name>` |

The step's `Screen` hands its sentence and actions to the dock with
`useShowInDock`. It is keyed by its slug and mounted outside the footer,
inside `OpeningCase` when that part exists. The frame reads dock updates
in a separate component, so handing up new words never redraws the
screen that supplied them.

The room's parts default-export their components. `bench/Bench.tsx`
mounts in `main.stage`, followed by `case-sheet/CaseSheet.tsx`.
`reopen-case/OpeningCase.tsx` wraps the step's screen. Missing parts leave
the frame whole. Tests hand in their own parts and need no bench.

The dock's primary stands down to the secondary look and gives up Enter
while the case sheet is open. A failed engine takes over the dock with
its fault title and a reload sentence. The notice retains
`data-testid="engine-status"`, `data-status` and the spoken
`data-testid="engine-version"`.

Tab follows the page: step options, badge, stage controls, dock controls.
The step control draws the registered steps. With no step after Find it
adds an unreachable 3D option; a registered later step takes that place.
Undo, Redo, Save and the case name are not mounted or wired here. Their earlier
components, key handling and tests remain in the tree, as do `StepRail`,
`GuideColumn`, `StatusLine` and their supporting readers.

`components.css` supplies the frame's parts. `AppFrame.css` only bounds
the root. The system’s tokens hide the product name at 600 wide or less.
No minimum width prevents the frame from shrinking.
