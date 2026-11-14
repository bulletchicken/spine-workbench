import { searchFilms, finderCapability } from '@/bench/search-films';
import { clearSpineBuild } from '@/bench/three/build-spine';
import { filmOrigins } from '@/view-state/film-origins';
import { findReplay } from '@/auto-pass/find-replay-clock';
import { revealPlan } from '@/auto-pass/find-replay';
import { caseStore } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { FILM_KINDS, marksOf } from '@/case-state/selectors';
import { motionMode, SYSTEM_ASKS_FOR_REDUCED } from '@/choreography/reduced-motion';
import { durationOnPage, numberOnPage } from '@/choreography/timing';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { compareLevels, type CaseFileV1, type FilmKind } from '@/shapes';
import { currentStep } from '@/view-state/current-step';
import { findState } from '@/view-state/find-state';
import { fitSources } from '@/view-state/fit-sources';
import { preferences } from '@/view-state/preferences';
import { selection } from '@/view-state/selection';
import { searchShare } from './search-progress';

interface Run { controller: AbortController; leaveStep(): void; leaveClock(): void }

/** The outlines a finished search is about: the Front film's, from T1 down. */
export function foundOnFront(c: CaseFileV1): number {
  return marksOf(c, 'front')?.vertebrae.filter((v) => compareLevels(v.level, 'T1') >= 0).length ?? 0;
}

export function foundOnLead(c: CaseFileV1): number {
  return foundOnFront(c) || (marksOf(c, 'side')?.vertebrae.length ?? 0);
}

/** Find with no search running shows what the case holds, and gives way to Films when the Front film has nothing found on it. */
export function showWhatIsFound(): void {
  if (foundOnLead(caseStore.getState().case) > 0) findState.getState().found();
  else { findState.getState().clear(); currentStep.getState().go('films'); }
}
let active: Run | null = null;
let clearing: ReturnType<typeof setTimeout> | null = null;

function cancelClear() {
  if (clearing !== null) clearTimeout(clearing);
  clearing = null;
}

function stop(afterFade = false) {
  cancelClear();
  const was = active;
  active = null;
  was?.leaveStep();
  was?.leaveClock();
  was?.controller.abort();
  selection.getState().hover(null);
  findState.getState().look(null);
  if (afterFade) {
    findReplay.getState().pause();
    clearing = setTimeout(() => {
      clearing = null;
      findReplay.getState().clear();
      findState.getState().rest();
      // Back in Find before the fade ended: the search that was stopped is over, and nothing would end its wait.
      if (currentStep.getState().slug === 'find') showWhatIsFound();
    }, durationOnPage('--dur-quick'));
  } else {
    findReplay.getState().clear();
    findState.getState().clear();
  }
}

function wait(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    const done = () => { clearTimeout(timer); signal.removeEventListener('abort', done); resolve(); };
    const timer = setTimeout(done, ms);
    signal.addEventListener('abort', done, { once: true });
    if (signal.aborted) done();
  });
}

async function search(engine: EngineClient, run: Run, only?: readonly FilmKind[]) {
  const info = await finderCapability(engine);
  if (active !== run) return;
  const openings = { ...filmPixels.getState().openings };
  const selection = searchFilms(caseStore.getState().case, filmOrigins.getState().films, openings, info);
  const films = FILM_KINDS.filter((kind) => only === undefined || only.includes(kind)).flatMap((kind) => selection.films.filter((film) => film.kind === kind));
  clearSpineBuild();
  // A kept but unsearched Side cannot reuse an earlier finding in the fit.
  if (!films.some((f) => f.kind === 'side')) {
    const side = caseStore.getState().case.films.find((f) => f.kind === 'side');
    if (side !== undefined && marksOf(caseStore.getState().case, 'side') !== null) caseStore.getState().apply([{ kind: 'set-film', film: side }]);
    fitSources.getState().forget('side');
  }
  const { case: c, version } = caseStore.getState();
  const { frontFinder, sideFinder } = selection;
  findState.getState().searching(films.length);
  findState.getState().searchedKinds(films.map((f) => f.kind));
  if (!films.some((f) => f.kind === 'front' || f.kind === 'side')) {
    findState.getState().clear();
    currentStep.getState().go('films');
    return;
  }
  const began = performance.now();
  const reduced = motionMode(window.matchMedia(SYSTEM_ASKS_FOR_REDUCED).matches, preferences.getState().motion === 'reduced') === 'reduced';
  const flight = reduced ? 2 * durationOnPage('--dur-fade') : durationOnPage('--dur-scene') + 3 * durationOnPage('--stagger');
  try {
    const result = await engine.runAutoPass({
      caseVersion: version, films,
      options: { acquisition: 'unknown', population: 'adolescent', template: null },
      firstVertebraId: c.nextVertebraId,
      frontFinder,
      sideFinder,
    }, {
      signal: run.controller.signal,
      onProgress: (progress) => { if (active === run) findState.getState().progress(searchShare(progress, films.length === 1 && films[0]?.kind === 'front')); },
    });
    if (active !== run) return;
    const currentMarks = result.marks.filter((marks) => films.some((film) => film.kind === marks.film) && openings[marks.film] === filmPixels.getState().openings[marks.film]);
    const report = caseStore.getState().apply(currentMarks.map((marks) => ({ kind: 'replace-marks', marks, basedOn: result.caseVersion, cause: { by: 'auto-pass' } })));
    caseStore.getState().startAfresh();
    const kept = currentMarks.filter((_, i) => report.outcomes[i]?.kept);
    for (const marks of kept) fitSources.getState().kept(marks.film, marks.film === 'front' ? frontFinder : marks.film === 'side' ? sideFinder : 'hand-built', openings[marks.film]);
    if (!kept.some((marks) => (marks.film === 'front' || marks.film === 'side') && marks.vertebrae.length > 0)) { showWhatIsFound(); return; }
    const sideResult = kept.find((m) => m.film === 'side');
    if (sideResult !== undefined) findState.getState().sideResult(sideResult);
    const sizes = Object.fromEntries(caseStore.getState().case.films.map((film) => [film.kind, { width: film.image.width, height: film.image.height }]));
    const plan = revealPlan(kept, sizes, { height: numberOnPage('--light-h'), edge: numberOnPage('--light-edge'), filmHeight: numberOnPage('--film-h') });
    if (plan === null) {
      findState.getState().failed('pass-no-spine-found');
      return;
    }
    await wait(Math.max(0, flight - (performance.now() - began)), run.controller.signal);
    if (active !== run) return;
    findState.getState().replaying();
    findReplay.getState().start(plan, { lead: durationOnPage('--dur-light-lead'), travel: durationOnPage('--dur-light'), hold: durationOnPage('--dur-hold') });
  } catch (error) {
    if (active !== run || !(error instanceof EngineFailure) || error.error.code === 'pass-cancelled') return;
    findReplay.getState().clear();
    findState.getState().failed(error.error.code);
  }
}

/** The run survives a screen's development remount; leaving the step owns cancellation. */
export const findRun = {
  start(engine: EngineClient, only?: readonly FilmKind[]): void {
    stop();
    const run: Run = { controller: new AbortController(), leaveStep: () => undefined, leaveClock: () => undefined };
    active = run;
    run.leaveStep = currentStep.subscribe((state) => { if (state.slug !== 'find' && active === run) stop(true); });
    run.leaveClock = findReplay.subscribe((state, was) => {
      if (active === run && was.playing && !state.playing && state.allShown) findState.getState().found();
    });
    void search(engine, run, only);
  },
  stop(): void { stop(); },
};
