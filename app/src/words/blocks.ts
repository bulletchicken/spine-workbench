// The words the building blocks say themselves: the two actions more than
// one area offers, and how a key hint writes each key.

export const BLOCK_WORDS = {
  close: 'Close',
  undo: 'Undo',
} as const;

/**
 * Keys a hint writes differently from the key's own name, by
 * `KeyboardEvent.key` name. Any other key is written as it is named
 * (Enter, Space, Home), a single letter in capitals.
 */
export const KEY_NAMES: Readonly<Record<string, string>> = {
  Escape: 'Esc',
  ArrowLeft: '←',
  ArrowRight: '→',
  ArrowUp: '↑',
  ArrowDown: '↓',
  '-': '−',
};

/**
 * The keys held with another key. An Apple keyboard prints signs on them
 * and a hint runs them together (⇧⌘Z); other keyboards print words, and a
 * hint joins them with a plus (Ctrl+Shift+Z). `Mod` is the key that goes
 * with Z to undo: Command on an Apple keyboard, Ctrl elsewhere.
 */
export const HELD_KEY_NAMES = {
  apple: { Mod: '⌘', Shift: '⇧', Alt: '⌥' },
  other: { Mod: 'Ctrl', Shift: 'Shift', Alt: 'Alt' },
} as const;
