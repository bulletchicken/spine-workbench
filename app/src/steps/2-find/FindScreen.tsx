import { useEffect, useRef } from 'react';
import { useStore } from 'zustand';
import { buildSpine, cancelSpineBuild } from '@/bench/three/build-spine';
import { spineBuild } from '@/view-state/spine-build';
import { THREE_WORDS } from '@/words/three';
import { findReplay } from '@/auto-pass/find-replay-clock';
import { ReplayPlayer } from '@/auto-pass/ReplayPlayer';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { caseStore, useCase } from '@/case-state/case-store';
import { durationOnPage } from '@/choreography/timing';
import { useEngine } from '@/engine-room/use-engine';
import { lookLevels } from '@/review/look-levels';
import { allSteps } from '@/steps/all-steps';
import { currentStep } from '@/view-state/current-step';
import { useShowInDock } from '@/view-state/dock-now';
import { filmIntake } from '@/view-state/film-intake';
import { fitSources } from '@/view-state/fit-sources';
import { findState, useFindState } from '@/view-state/find-state';
import { passRequest } from '@/view-state/pass-request';
import { selection } from '@/view-state/selection';
import { findDock, type FindActions } from './find-dock';
import { findRun, foundOnLead, showWhatIsFound } from './find-run';

export function FindScreen() {
  const engine = useEngine();
  const build = useStore(spineBuild);
  const trainedSide = useStore(fitSources, (s) => s.films.side?.finder === 'trained');
  useEffect(() => () => { cancelSpineBuild(); }, []);
  const entered = useRef(false);
  const find = useFindState();
  const slowSearch = useStore(filmIntake, (state) => state.from === 'made-up-case');
  const films = useCase((state) => state.case.marks);
  const inPlace = useCase((state) => state.case.films.length);
  const plan = useStore(findReplay, (state) => state.plan);
  const revealed = useStore(findReplay, (state) => state.revealed);

  useEffect(() => {
    if (entered.current) return;
    entered.current = true;
    if (passRequest.getState().take()) findRun.start(engine);
    else if (['searching', 'replaying'].includes(findState.getState().phase)) return;
    else showWhatIsFound();
  }, [engine]);
  const onFront = useCase((state) => foundOnLead(state.case));
  useEffect(() => {
    // The Front film was replaced under a finished search: nothing here was found on the film now in place.
    if (find.phase === 'found' && onFront === 0) showWhatIsFound();
  }, [find.phase, onFront]);
  useEffect(() => {
    if (!find.holding || find.review === null) return;
    const next = find.review + 1;
    const timer = setTimeout(() => { findState.getState().look(next); }, durationOnPage('--dur-hold'));
    return () => { clearTimeout(timer); };
  }, [find.holding, find.review]);
  useEffect(() => () => { findState.getState().look(null); selection.getState().hover(null); }, []);
  useKey('review', ['Escape'], () => { findState.getState().look(null); }, () => findState.getState().review !== null);
  const act: FindActions = {
    review: () => {
      selection.getState().hover(null);
      const state = findState.getState();
      findState.getState().openReview(lookLevels(caseStore.getState().case.marks.filter((m) => !trainedSide || m.film !== 'side')).filter((entry) => !state.looked.includes(entry.level)).map((entry) => entry.level));
    },
    looksRight: () => {
      const state = findState.getState();
      const level = state.review === null ? undefined : state.reviewing[state.review];
      if (!state.holding && level !== undefined) state.lookedAt(level);
    },
    backToFilms: () => { currentStep.getState().go('films'); },
    tryAgain: () => { findRun.start(engine, findState.getState().kinds.length > 0 ? findState.getState().kinds : undefined); },
    frontAlone: () => { findRun.start(engine, ['front']); },
  };
  const later = allSteps().find((step) => step.number > 2);
  useShowInDock(build.phase === 'building' ? { scene: 'building', sentence: THREE_WORDS.building } : build.phase === 'failed' ? {
    scene: 'build-failed', sentence: THREE_WORDS.failed, primary: { label: THREE_WORDS.retry, press: () => { void buildSpine(engine); } },
  } : findDock({ trainedSide, find, films, inPlace, replay: { ...findReplay.getState(), plan, revealed }, slowSearch, laterStep: later === undefined ? null : { label: later.title, press: () => { if (later.slug === 'three') void buildSpine(engine); else currentStep.getState().go(later.slug); } } }, act));
  return <ReplayPlayer clock={findReplay} />;
}
