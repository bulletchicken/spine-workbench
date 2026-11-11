import './Kbd.css';
import { keyHint } from './keyboard/key-hint';

interface KbdProps {
  /**
   * The key, or the keys pressed together, as `KeyboardEvent.key` names
   * plus `Mod`, `Shift` and `Alt`: `['Enter']`, `['Mod', 'z']`.
   */
  keys: readonly string[];
  /** `chip` sits in a small tinted box; `bare` is the letters alone, for a hint tucked beside a word. */
  look?: 'chip' | 'bare';
  className?: string;
}

/**
 * A key hint: the key that does what the control beside it does, written
 * the way this machine's keyboard prints it.
 *
 * It takes its colours from `--kbd-ink` and `--kbd-fill`, so a part on a
 * surface of its own (the lit button, which is solid ink) sets those two
 * and the hint fits.
 */
export function Kbd({ keys, look = 'chip', className }: KbdProps) {
  return (
    <kbd className={className === undefined ? 'kbd' : `kbd ${className}`} data-look={look}>
      {keyHint(keys)}
    </kbd>
  );
}
