# guide

The dock's one sentence and next action. Everything here draws from
props: nothing reads the case, view state or engine.

| File | Holds |
|---|---|
| `dock-spec.ts` | the eight scenes, helper kinds and optional quiet and primary actions |
| `Dock.tsx` | one footer with all eight scenes, and one spoken status line |
| `Dock.css` | the one look the system's dock does not cover: a wait's meter that has stood still |
| `LitButton.tsx` | the primary button, or the secondary look while a sheet has the primary |
| `lit-stands-down.ts` | the frame's word that a sheet has the primary for now |

A step calls `useShowInDock` in `view-state/dock-now.ts`. The frame mounts
one `Dock` and passes it the current spec. The eight scenes are `add`,
`reading`, `ready`, `finding`, `found`, `review`, `failed` and `engine`.
Only one has `on`; the others are inert and hidden from assistive
technology. Each keeps its last words and controls while it fades out.
Its presses and Enter handling are asleep as soon as it stops being on.

Within a scene, quiet and primary buttons keep their nodes and focus
when labels change. New sentence and helper words fade in. The spoken
status line changes only when the words read differently; a running
count and progress meter stay outside it. A new meter replaces the
waiting meter when the outline count starts.

Enter presses the current enabled primary once, unless a focused
control keeps Enter for itself. Disabled and sleeping buttons take no
press. A sheet makes the dock primary secondary and takes its Enter.
No key hint is drawn on the button. A spec with no primary offers no
Enter action. The found scene can therefore offer its quiet review
action alone while there is no later moment to open.

The helper may hold text and links, an amber dot with words, a meter
with words, or a mono count and meter. Every sentence and helper stays
on one line, ending with an ellipsis before it can touch a button. At
900 wide or less the sentence uses Body, medium. The design system’s
classes and tokens supply these rules and every size, colour, surface and
transition.

The earlier `Guide`, `guide-spec.ts`, `ChoiceCards`, `QuietActions`,
`HalfFilmFigure` and `cards/` remain available with their tests and
gallery pages. They are no longer mounted in the one stage frame.
Their choices keep their own keys and their quiet actions keep focus
when offered again. `LitButton` still accepts their optional icon.
