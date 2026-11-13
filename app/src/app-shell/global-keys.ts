// The keys that answer on every screen, at the highest rank: Cmd or
// Ctrl+Z to undo, with Shift to redo, and Cmd or Ctrl+S to save. They are
// asked first, and are the only keys asked while another key is held or
// focus is in a text field.

import { useKey } from '@/building-blocks/keyboard/use-key';
import { openCaseSheet, redoOnce, undoOnce } from './frame-actions';

interface GlobalKeys {
  /** Asked at each press: undo and redo answer here. */
  undoIsLive: () => boolean;
  /** Asked at each press: the Save case button is live. */
  saveIsLive: () => boolean;
}

const heldWithCommand = (event: KeyboardEvent): boolean => (event.metaKey || event.ctrlKey) && !event.altKey;

/** Register the frame's keys for as long as the frame is mounted. */
export function useGlobalKeys({ undoIsLive, saveIsLive }: GlobalKeys): void {
  // Where undo is not live the key is not ours: a text field keeps its own undo.
  useKey('always', ['z'], (event) => {
    if (!heldWithCommand(event) || !undoIsLive()) return false;
    if (event.shiftKey) redoOnce();
    else undoOnce();
    return true;
  });

  // Cmd or Ctrl+S is always taken, so the browser never offers to save the
  // page itself; the sheet opens only when there is a case to save.
  useKey('always', ['s'], (event) => {
    if (!heldWithCommand(event) || event.shiftKey) return false;
    if (saveIsLive() && !event.repeat) openCaseSheet();
    return true;
  });
}
