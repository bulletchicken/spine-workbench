import { say } from '@/building-blocks/toast/say';
import { caseStore } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { openAFilm } from '@/open-and-save/open-a-film';
import { readBytes } from '@/open-and-save/read-bytes';
import { currentStep } from '@/view-state/current-step';
import { filmIntake } from '@/view-state/film-intake';
import { filmOrigins } from '@/view-state/film-origins';
import { findState } from '@/view-state/find-state';
import { fitSources } from '@/view-state/fit-sources';
import { savedState } from '@/view-state/saved-state';
import { SHELL_WORDS } from '@/words/shell';
import { clearFilms } from './start-over';
import { reopenedSources } from './reopened-sources';

/** Called in the film-change queue. Strict validation finishes before the current case is cleared. */
export async function openSavedCase(engine: EngineClient, blob: Blob): Promise<void> {
  const opened = await engine.openCase(await readBytes(blob));
  currentStep.getState().go('films');
  clearFilms(engine);
  filmIntake.getState().source('yours');
  filmIntake.getState().begin(opened.case.films.map((film) => film.kind));
  try {
    for (const film of opened.case.films) {
      const png = opened.films.find((file) => file.kind === film.kind);
      if (png === undefined) throw new EngineFailure({ code: 'case-missing-film', message: 'a saved film is missing', detail: null });
      await openAFilm(engine, film.kind, new Uint8Array(png.png).buffer, film);
      filmOrigins.getState().opened(film.kind, 'yours', filmPixels.getState().openings[film.kind]);
      filmIntake.getState().read(film.kind);
    }
    caseStore.getState().apply([{ kind: 'open-case', case: opened.case }]);
    const sources = reopenedSources(opened.case, filmPixels.getState().openings);
    for (const film of opened.case.films) {
      const source = sources[film.kind];
      if (source !== undefined) fitSources.getState().kept(film.kind, source.finder, source.opening);
    }
    const side = opened.case.marks.find((marks) => marks.film === 'side');
    if (side !== undefined) findState.getState().sideResult(side);
    findState.getState().searchedKinds(opened.case.marks.map((marks) => marks.film));
    savedState.getState().markOpened(caseStore.getState().version);
    if (opened.case.marks.some((marks) => marks.vertebrae.length > 0)) {
      findState.getState().found();
      currentStep.getState().go('find');
    }
    say(SHELL_WORDS.openedCase);
  } catch (error) {
    // A resource failure after validation must not leave a half-open case on the stage.
    clearFilms(engine);
    throw error;
  }
}
