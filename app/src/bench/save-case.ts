import { say } from '@/building-blocks/toast/say';
import { caseStore } from '@/case-state/case-store';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { chooseCaseDestination } from '@/open-and-save/save-file';
import type { CaseFileV1 } from '@/shapes';
import { savedState } from '@/view-state/saved-state';
import { faultWords } from '@/words/faults';
import { SHELL_WORDS } from '@/words/shell';
import { queueFilmChange } from './start-over';

export function hasCaseToSave(c: CaseFileV1): boolean {
  return c.marks.some((marks) => marks.vertebrae.length > 0 && c.films.some((film) => film.kind === marks.film));
}

/** The engine writes the file; only the local destination receives its bytes. */
export async function saveCurrentCase(engine: Pick<EngineClient, 'saveCase'>): Promise<void> {
  const { case: c, version } = caseStore.getState();
  let choosing = true;
  try {
    const write = await chooseCaseDestination();
    choosing = false;
    await queueFilmChange(async () => {
      // The dialog or an earlier queued opening may outlive the case the press was for.
      if (caseStore.getState().version !== version) {
        say(SHELL_WORDS.caseChangedBeforeSave, { tone: 'note' });
        return;
      }
      if (!hasCaseToSave(c)) return;
      const bytes = await engine.saveCase(c);
      const result = await write(bytes);
      if (result === 'written') {
        savedState.getState().markSaved(version);
        say(SHELL_WORDS.caseSaved);
      } else say(SHELL_WORDS.caseDownloaded);
    });
  } catch (error) {
    if (choosing && error instanceof DOMException && error.name === 'AbortError') return;
    if (error instanceof EngineFailure && error.error.code === 'pass-cancelled') return;
    say(error instanceof EngineFailure ? faultWords(error.error.code).title
      : `${SHELL_WORDS.caseWriteFailed} ${SHELL_WORDS.caseWriteFailedHelp}`, { tone: 'note' });
  }
}
