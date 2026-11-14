import { clearSpineBuild } from './three/build-spine';
import { fitSources } from '@/view-state/fit-sources';
import { caseStore } from '@/case-state/case-store';
import { emptyCase } from '@/case-state/empty-case';
import { filmPixels } from '@/case-state/film-pixels';
import { FILM_KINDS } from '@/case-state/selectors';
import { durationOnPage } from '@/choreography/timing';
import type { EngineClient } from '@/engine-room/engine-client';
import { popoverOpen } from '@/on-this-device/popover-open';
import { currentStep } from '@/view-state/current-step';
import { filmIntake } from '@/view-state/film-intake';
import { passRequest } from '@/view-state/pass-request';
import { selection } from '@/view-state/selection';
import { findReplay } from '@/auto-pass/find-replay-clock';
import { findState } from '@/view-state/find-state';

let waiting = Promise.resolve();

/** Clearing and opening share a queue, so late pixels cannot refill an emptied stage. */
export function queueFilmChange(change: () => Promise<void>): Promise<void> {
  const next = waiting.then(change);
  waiting = next.catch(() => undefined);
  return next;
}

/** Samples and personal films never share the stage. This reset has no fade or step change. */
export function clearFilms(engine: Pick<EngineClient, 'retireFilm'>): void {
  clearSpineBuild();
  fitSources.getState().clear();
  for (const kind of FILM_KINDS) {
    engine.retireFilm(kind);
    filmPixels.getState().retired(kind);
  }
  caseStore.getState().apply([{ kind: 'open-case', case: emptyCase() }]);
  filmIntake.getState().clear();
  findReplay.getState().clear();
  findState.getState().clear();
  selection.getState().select(null);
  selection.getState().hover(null);
  passRequest.getState().take();
}

export function startOver(engine: Pick<EngineClient, 'retireFilm'>): Promise<void> {
  return queueFilmChange(async () => {
    filmIntake.getState().leave(true);
    popoverOpen.getState().setOpen(false);
    await new Promise<void>((resolve) => { setTimeout(resolve, durationOnPage('--hold-leave')); });
    clearFilms(engine);
    currentStep.getState().go('films');
    filmIntake.getState().leave(false);
  });
}
