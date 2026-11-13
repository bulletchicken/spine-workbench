// What the top bar's buttons and the keys that go with them do. Each acts
// on the app's own stores, so a press and a key are one action.

import { say } from '@/building-blocks/toast/say';
import { caseStore } from '@/case-state/case-store';
import { caseSheetOpen } from '@/view-state/case-sheet-open';
import { redidLine, undidLine } from '@/words/shell';

/** Take back the newest action, and say what it was. Nothing happens when there is none. */
export function undoOnce(): void {
  const undone = caseStore.getState().undo();
  if (undone !== null) say(undidLine(undone));
}

/** Bring back the newest undone action, and say what it was. */
export function redoOnce(): void {
  const redone = caseStore.getState().redo();
  if (redone !== null) say(redidLine(redone));
}

/** Open the case sheet. */
export function openCaseSheet(): void {
  caseSheetOpen.getState().setOpen(true);
}
