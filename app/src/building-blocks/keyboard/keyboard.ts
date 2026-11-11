// The app's one key listener. Every part that answers a key says so here,
// under a rank (key-ranks.ts), and no other file listens for keys on the
// document or the window.
//
// When a key goes down the ranks are walked from the highest, and within a
// rank from the newest taker to the oldest. The first taker whose `run`
// does not return `false` has the key: its default is prevented and the
// walk stops. Five rules decide who is asked at all:
//
// 1. A key a focused control has already handled is nobody else's. A
//    control that answers its own keys (a slider's arrows, a pin's, a
//    rung's) calls `preventDefault`, and the walk never starts.
// 2. Enter and Space on a focused button or link press that control, so
//    Enter on a focused button never presses the lit one as well. In the
//    same way a checkbox, a radio button and a range input keep the keys
//    the browser answers on them. Any other part that can hold focus and
//    must keep Enter from the lit button (a pin, a rung) takes Enter
//    itself, by rule 1.
// 3. A key held with Cmd, Ctrl or Alt is asked of `always` only.
// 4. While focus is in a field that takes typing, only `always` is asked.
// 5. While a part stops the keys at its rank (an open dialog, at `sheet`),
//    no rank below it is asked: a key nobody above takes goes to nobody.

import { KEY_RANKS, type KeyRank } from './key-ranks';

/**
 * What a taker does with a key. Returning `false` means "not mine", and
 * the walk goes on; returning anything else, or nothing, takes the key.
 */
export type KeyRun = (event: KeyboardEvent) => unknown;

interface Taker {
  rank: KeyRank;
  keys: ReadonlySet<string>;
  run: KeyRun;
  when: (() => boolean) | undefined;
}

/** Oldest first. */
const takers: Taker[] = [];

/** The ranks at which the keys are stopped just now, one for each part stopping them. */
const stops: KeyRank[] = [];

const TAKES_TYPING = 'textarea, select, [contenteditable]:not([contenteditable="false"])';
const PRESSES_ITSELF = 'button, a[href], summary, [role="button"], [role="link"]';
const PRESSING_KEYS = ['Enter', 'Space'];
const ARROW_KEYS = ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'];

// The kinds of input that are not typed into, each with the keys the
// browser itself answers on it. An input of any other kind is a field
// that takes typing.
const INPUT_KEEPS = new Map<string, readonly string[]>([
  ['checkbox', ['Space']],
  ['radio', ['Space', ...ARROW_KEYS]],
  ['range', [...ARROW_KEYS, 'Home', 'End', 'PageUp', 'PageDown']],
  ...['button', 'submit', 'reset', 'image', 'file', 'color'].map((kind) => [kind, PRESSING_KEYS] as const),
]);

/** The keys that are the focused control's own: the browser answers them there, so the walk leaves them alone. */
function ownKeys(focused: Element): readonly string[] {
  if (focused instanceof HTMLInputElement) return INPUT_KEEPS.get(focused.type) ?? [];
  return focused.closest(PRESSES_ITSELF) === null ? [] : PRESSING_KEYS;
}

function takesTyping(focused: Element): boolean {
  if (focused instanceof HTMLInputElement) return !INPUT_KEEPS.has(focused.type);
  return focused.closest(TAKES_TYPING) !== null;
}

/** The ranks a key is asked of: all of them, or down to the highest rank the keys are stopped at. */
function ranksAsked(): readonly KeyRank[] {
  if (stops.length === 0) return KEY_RANKS;
  return KEY_RANKS.slice(0, Math.min(...stops.map((rank) => KEY_RANKS.indexOf(rank))) + 1);
}

/** One spelling for a key: `Space` for the space bar, a lone character in lower case, any other name as the browser gives it. */
export function keyName(key: string): string {
  if (key === ' ' || key === 'Spacebar') return 'Space';
  return key.length === 1 ? key.toLowerCase() : key;
}

function onKeyDown(event: KeyboardEvent): void {
  if (event.defaultPrevented || event.isComposing) return;
  const key = keyName(event.key);
  const focused = event.target instanceof Element ? event.target : null;
  const held = event.metaKey || event.ctrlKey || event.altKey;
  if (!held && focused !== null && ownKeys(focused).includes(key)) return;
  const alwaysOnly = held || (focused !== null && takesTyping(focused));

  // A copy, newest first: a taker may close the part that registered it.
  const asked = [...takers].reverse();
  for (const rank of alwaysOnly ? (['always'] as const) : ranksAsked()) {
    for (const taker of asked) {
      if (taker.rank !== rank || !taker.keys.has(key)) continue;
      if (taker.when !== undefined && !taker.when()) continue;
      if (taker.run(event) === false) continue;
      event.preventDefault();
      return;
    }
  }
}

/**
 * Answer `keys` at `rank` until the function returned is called. `keys`
 * are `KeyboardEvent.key` names; letters match in either case and the
 * space bar is `Space`. `when`, if given, is asked at each press: the
 * taker is passed over while it answers `false`.
 */
export function takeKeys(rank: KeyRank, keys: readonly string[], run: KeyRun, when?: () => boolean): () => void {
  const taker: Taker = { rank, keys: new Set(keys.map(keyName)), run, when };
  if (takers.length === 0) document.addEventListener('keydown', onKeyDown);
  takers.push(taker);
  return () => {
    const at = takers.indexOf(taker);
    if (at === -1) return;
    takers.splice(at, 1);
    if (takers.length === 0) document.removeEventListener('keydown', onKeyDown);
  };
}

/**
 * Stop every key at `rank` until the function returned is called: no rank
 * below it is asked, and a key nobody at `rank` or above takes goes to
 * nobody, with its default left alone. For a part that must be answered
 * before anything behind it, as an open dialog must.
 */
export function stopKeysAt(rank: KeyRank): () => void {
  stops.push(rank);
  let stopping = true;
  return () => {
    if (!stopping) return;
    stopping = false;
    stops.splice(stops.indexOf(rank), 1);
  };
}
