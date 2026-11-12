# building-blocks

The small pieces every screen is assembled from, and the app's one key
listener. A block knows nothing about films or cases: it takes props and
draws. Each has a page in the gallery (`?gallery` in development).

| Block | File | What it is |
|---|---|---|
| `Button` | `Button.tsx` | a button that is not the next action, wearing the design system's class: `standard` is `.secondary`, a white pill with a hairline edge; `text` is `.quiet`, words only. `regular` or `large`, with disabled and busy states |
| `IconButton` | `IconButton.tsx` | an icon with a spoken name and a tooltip that can name the action a press would do |
| `Kbd` | `Kbd.tsx` | a key hint, written as this machine's keyboard prints it (⌘Z or Ctrl+Z) |
| `Tooltip` | `Tooltip.tsx` | a few words beside a control, on hover and on keyboard focus |
| `Popover` | `Popover.tsx` | a panel beside the control that opened it, always on the page; `open` adds the class `open`, and the caller's classes (the design system's `note glass`) give its look and both fades. Escape, a press outside or focus moving on closes it |
| `Segmented` | `Segmented.tsx` | the design system's `.seg`: a pill of options with a white thumb that slides under the chosen one, the steps of the journey |
| `Dialog` | `Dialog.tsx` | a sheet of paper over a veil, with focus kept inside it and every key kept from what is behind it |
| `Slider` | `Slider.tsx` | a slider cut into named parts, for the pointer and the keyboard |
| `Toasts`, `say()` | `toast/` | one line at the foot of the bench saying what just happened, with at most one action |
| `useKey` | `keyboard/` | how a part answers keys |

## Rules the blocks keep

- **An icon is a child.** A block that shows an icon is handed it; none
  imports or draws one. The toasts are handed the two marks a line can
  open with by whoever mounts them:
  `<Toasts marks={{ done: <tick>, note: <info> }} />`.
- **Nothing is moved to the end of the page.** A tooltip, a popover, a
  dialog and the toasts are drawn where they are mounted and placed
  against their nearest positioned ancestor. That is how a dialog's veil
  covers one region and not the window, and it keeps every block inside
  the app root, where `data-motion` and the tokens reach it.
- **A block wears the design system's class** where the system draws the
  part (`quiet`, `secondary`, `seg`; the popover takes its caller's), and
  `app/src/look-and-feel/components.css` holds the rules. A block's own
  stylesheet holds only what the system does not cover.
- **State reaches CSS through classes and attributes** (`open`,
  `data-size`, `data-none`, `aria-pressed`, `aria-expanded`,
  `aria-current`, `aria-disabled`, `aria-busy`), never through a style set
  from script.
  The slider sets only `--span` and `--p`, and the step control only
  `--at`, which are geometry.
- **Every duration is a token.** With `data-motion="reduced"` a popover
  fades in and out without moving, the step control's thumb jumps to its option,
  a dialog and a toast are there at once and the busy ring stands still.
  How long a toast stays is `--t-toast-stay`, which is not motion and is
  never shortened.
- **Two animations are clocks.** A dialog is taken off the page when
  `dialog-out` ends on its sheet, and a toast is put away when
  `toast-stay` ends. A stylesheet that cancels either (a blanket
  `animation: none` for reduced motion is the likely way) leaves the
  sheet and its veil on the page for good, taking no press and no focus,
  or the toast up for ever. So no rule outside this folder sets
  `animation` on `.dialog`, `.dialog-veil` or `.toast-clock`. Reduced
  motion shortens the dialog's leaving through `--t-base` and leaves the
  toast's stay alone.
- **A press is `onPress`.** A disabled `Button` leaves the tab order. A
  disabled `IconButton` stays in it, so its tooltip can say why. A busy
  button keeps its focus and its width.
- **A wait is said in words.** A busy button hides its words behind a
  turning ring, and with reduced motion the ring stands still. That is
  enough for the moment a press takes; it is not how a wait is told. The
  part that makes a button busy says what is happening in words beside
  it ("Packing four films and the data file…") or through `say()`.

A surface of its own sets a block's colours through custom properties
instead of restyling it: `--kbd-ink`, `--kbd-fill` and `--kbd-edge` for
a key hint on the lit button, `--slider-segment-min` for a slider whose
parts need room for their captions.

## The keyboard

One `keydown` listener on the document, in `keyboard/keyboard.ts`. No
other file listens for keys on the document or the window. A part says
which keys it answers, under a rank:

```ts
useKey('lit', ['Enter'], () => press());
useKey('popover', ['Escape'], close, () => open);
useKey('always', ['z'], (event) => {
  if (!(event.metaKey || event.ctrlKey)) return false; // not mine
  undo();
});
```

Keys are `KeyboardEvent.key` names; letters match in either case and the
space bar is `Space`. When a key goes down the ranks are walked in the
order of `keyboard/key-ranks.ts`, highest first, and within a rank from
the part mounted last to the one mounted first. The first `run` that
does not return `false` takes the key: its default is prevented and the
walk stops.

Who is asked at all:

1. A key a focused control has handled itself is nobody else's. A control
   that answers its own keys (a slider's arrows, a pin's, a rung's) calls
   `preventDefault` in its own `onKeyDown`, and the walk never starts.
2. Enter and Space on a focused button or link press that control, so
   Enter on a focused button never presses the lit one as well. A
   checkbox, a radio button and a range input keep the keys the browser
   answers on them in the same way (Space; the arrows; Home and End).
3. A key held with Cmd, Ctrl or Alt is asked of `always` only.
4. While focus is in a field that takes typing (a text input of any
   kind, a textarea, a select, editable text), only `always` is asked.
5. While a dialog is open, only `always` and `sheet` are asked: no key
   reaches a part behind the veil, and undo and save still answer.

Rule 2 covers buttons, links and those three inputs, and nothing else.
**A pin or a rung that has focus must take Enter itself**, in its own
`onKeyDown` with `preventDefault` (rule 1); otherwise Enter walks on and
presses the lit button.

Rule 4 holds inside a dialog and a popover too: with focus in a text
field, Escape is not asked of `sheet` or `popover`, so it does not close
them. No dialog or popover holds a field yet. The first one that does
closes itself from that field's own `onKeyDown`, as a modal dialog is
expected to on Escape.

Rule 5 is `stopKeysAt(rank)` in `keyboard/keyboard.ts`, which `Dialog`
calls for as long as it is open and not on its way out. A key nobody at
that rank or above takes then goes to nobody, and keeps its default.

`run` is handed the event, so it can read `event.repeat` and the keys
held with it.

## Tests

`test-rig/on-the-page.tsx` mounts a part in jsdom and sends it real DOM
events: keys, presses, focus, the pointer coming and going, the end of an
animation. It is for `*.test.tsx` files in any area.
