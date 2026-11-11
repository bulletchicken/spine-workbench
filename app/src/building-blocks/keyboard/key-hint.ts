// How a key, or a key held with others, is written in a key hint: "Enter",
// "⌘Z" on an Apple keyboard, "Ctrl+Z" on any other. The names are in
// words/blocks.ts.

import { HELD_KEY_NAMES, KEY_NAMES } from '@/words/blocks';
import { keyName } from './keyboard';

type HeldKey = keyof typeof HELD_KEY_NAMES.apple;

// The order each kind of keyboard writes its held keys in: Apple menus
// read ⌥⇧⌘, the others Ctrl+Alt+Shift.
const HELD_ORDER = {
  apple: ['Alt', 'Shift', 'Mod'],
  other: ['Mod', 'Alt', 'Shift'],
} as const satisfies Record<string, readonly HeldKey[]>;

/** True when this machine's keyboard has a Command key. */
export const ON_AN_APPLE_KEYBOARD = typeof navigator !== 'undefined' && /\b(Macintosh|iPhone|iPad)\b/.test(navigator.userAgent);

/**
 * The hint for `keys` pressed together, given as `KeyboardEvent.key`
 * names plus `Mod`, `Shift` and `Alt` for the keys held with them.
 */
export function keyHint(keys: readonly string[], apple: boolean = ON_AN_APPLE_KEYBOARD): string {
  const kind = apple ? 'apple' : 'other';
  const held = HELD_ORDER[kind].filter((key) => keys.includes(key)).map((key) => HELD_KEY_NAMES[kind][key]);
  const pressed = keys
    .filter((key) => !(key in HELD_KEY_NAMES.apple))
    .map((key) => {
      const name = keyName(key);
      return KEY_NAMES[name] ?? (name.length === 1 ? name.toUpperCase() : name);
    });
  return [...held, ...pressed].join(apple ? '' : '+');
}
