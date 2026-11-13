import { IconButton } from '@/building-blocks/IconButton';
import type { ActionRecord } from '@/case-state/action-record';
import { Icon } from '@/look-and-feel/icons/Icon';
import { redoTip, SHELL_WORDS, undoTip } from '@/words/shell';

interface UndoRedoProps {
  /** Undo and redo answer here. Where they do not, both say there is nothing to take back. */
  live: boolean;
  /** What a press of each would act on, or `null` when there is nothing. */
  nextUndo: ActionRecord | null;
  nextRedo: ActionRecord | null;
  onUndo: () => void;
  onRedo: () => void;
}

/**
 * Undo and redo: two icon buttons that are never icons alone. Each has a
 * tooltip that names the very action a press would take back or bring
 * back ("Undo: Split T12"), or says there is nothing to.
 */
export function UndoRedo({ live, nextUndo, nextRedo, onUndo, onRedo }: UndoRedoProps) {
  const undo = live ? nextUndo : null;
  const redo = live ? nextRedo : null;
  return (
    <>
      <IconButton label={SHELL_WORDS.undo} tip={undoTip(undo)} tipAlign="end" disabled={undo === null} onPress={onUndo}>
        <Icon name="undo" />
      </IconButton>
      <IconButton label={SHELL_WORDS.redo} tip={redoTip(redo)} tipAlign="end" disabled={redo === null} onPress={onRedo}>
        <Icon name="redo" />
      </IconButton>
    </>
  );
}
