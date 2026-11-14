import type { FilmKind } from '@/shapes';
import { useEffect, useLayoutEffect, useMemo, useState } from 'react';
import { createPortal } from 'react-dom';
import { useSpineHost } from '@/app-shell/spine-host';
import { loadBones } from '@/spine-3d/bones/load-bones';
import { currentStep } from '@/view-state/current-step';
import { spineBuild } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import { THREE_WORDS } from '@/words/three';
import { useStore } from 'zustand';
import { useCase } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { useCurrentStep } from '@/view-state/current-step';
import { useFilmIntakeState } from '@/view-state/film-intake';
import { useFilmIntake } from './film-intake';
import { SideReview } from '@/review/SideReview';
import { renamedSide, sideReviewKey, type SideNameChange } from '@/review/side-review';
import { filmOrigins } from '@/view-state/film-origins';
import { findState } from '@/view-state/find-state';
import { fitSources } from '@/view-state/fit-sources';
import { caseStore } from '@/case-state/case-store';
import { useEngine } from '@/engine-room/use-engine';
import { clearSpineBuild } from './three/build-spine';
import { SideFacing } from './SideFacing';
import { exclusionWords, useSearchFilms } from './search-films';
import { Stage, type StageFilm, type FilmDriver } from './Stage';
import { FILM_ORDER } from './stage-layout';
import { useFindOnStage } from './find-on-stage';
import { hasCurrentFit } from './three/build-spine';
import { markAtLevel } from './three/spine-links';
import { Measurements } from './three/Measurements';
import { PlaceSheet } from './place/PlaceSheet';
import { placeState } from './place/place-state';
import { FilmCredit } from './FilmCredit';
import { ThreeStage } from './three/ThreeStage';

/** The stage stays mounted as the step changes, so each film keeps its tile. */
export default function Bench() {
  const engine = useEngine();
  const review = useStore(findState);
  const sideSource = useStore(fitSources, (s) => s.films.side);
  const sideRevision = useCase((s) => s.changedAt.side);
  const { held, pressPlace } = useFilmIntake();
  const intake = useFilmIntakeState();
  const { slug } = useCurrentStep();
  const c = useCase((state) => state.case);
  const records = c.films;
  const threeView = useStore(threeState);
  const build = useStore(spineBuild);
  const host = useSpineHost();
  const valid = hasCurrentFit(c);
  const placeView = useStore(placeState);
  const placing = slug === 'place';
  const three = (slug === 'three' || placing) && valid && build.accepted !== null;
  const [filmDriver, setFilmDriver] = useState<FilmDriver | null>(null);
  const [bonesReady, setBonesReady] = useState(false);
  useEffect(() => {
    let mounted = true;
    void loadBones().then(() => { if (mounted) setBonesReady(true); }, () => undefined);
    return () => { mounted = false; };
  }, []);
  useLayoutEffect(() => {
    if ((slug === 'three' || slug === 'place') && !valid) currentStep.getState().go(c.marks.some((f) => (f.film === 'front' || f.film === 'side') && f.vertebrae.length > 0) ? 'find' : 'films');
  }, [slug, valid, c]);
  useEffect(() => {
    const stage = host?.parentElement?.querySelector('main.stage');
    if (build.busy) stage?.setAttribute('aria-busy', 'true');
    else stage?.removeAttribute('aria-busy');
    return () => { stage?.removeAttribute('aria-busy'); };
  }, [build.busy, host]);
  const pixels = useStore(filmPixels, (state) => state.films);
  const finding = useFindOnStage();
  const search = useSearchFilms();
  const sideOrigin = useStore(filmOrigins, (s) => s.films.side?.from);
  const sideOpening = useStore(filmPixels, (s) => s.openings.side);
  const films = useMemo(() => Object.fromEntries(FILM_ORDER.map((kind) => {
    const record = records.find((film) => film.kind === kind);
    const opened = pixels[kind];
    const film: StageFilm | null = record === undefined || opened === undefined ? null : {
      ...(kind === 'side' ? { view: { quarterTurns: record.geometry.quarterTurns, mirrored: false } } : {}),
      size: { width: record.image.width, height: record.image.height }, ...opened,
      outlines: [], names: slug === 'find' && (kind === 'front' || kind === 'side'), on: null, only: null, frame: { kind: 'whole' }, light: null, ...finding.films[kind],
    };
    if (three && film !== null) film.on = markAtLevel(film.outlines, threeView.rowLevels[0] ?? threeView.pointed);
    return [kind, film];
  })) as Record<(typeof FILM_ORDER)[number], StageFilm | null>, [records, pixels, slug, finding.films, three, threeView.pointed, threeView.rowLevels]);
  const sideMarks = c.marks.find((m) => m.film === 'side');
  const reviewSide = slug === 'find' && review.phase === 'found' && sideSource?.finder === 'trained' && sideSource.opening === sideOpening && sideMarks !== undefined;
  const renameSide = (change: SideNameChange) => {
    if (sideMarks === undefined || review.sideBaseline === null) return;
    const baseline = review.sideBaseline;
    const renamed = renamedSide(baseline, change);
    if (JSON.stringify(sideMarks) === JSON.stringify(renamed)) return;
    clearSpineBuild();
    const last = caseStore.getState().history.undo.at(-1);
    if (last?.after.side?.marks === sideMarks && JSON.stringify(last.before.side?.marks) === JSON.stringify(baseline)) caseStore.getState().undo();
    const { version } = caseStore.getState();
    caseStore.getState().apply([{ kind: 'replace-marks', marks: renamed, basedOn: version, cause: { by: 'auto-pass' } }]);
  };
  const leaveSideOut = () => {
    clearSpineBuild();
    engine.retireFilm('side'); filmPixels.getState().retired('side');
    fitSources.getState().forget('side'); filmOrigins.getState().forget('side');
    caseStore.getState().apply([{ kind: 'remove-film', film: 'side' }]);
    findState.getState().approveSide('');
    findState.getState().look(null);
    if (!caseStore.getState().case.marks.some((m) => m.film === 'front' && m.vertebrae.length > 0)) {
      findState.getState().clear(); currentStep.getState().go('films');
    }
  };
  return <>
    <span className="spoken-only" data-testid="bones-status" data-status={bonesReady ? 'ready' : 'loading'}>{bonesReady ? THREE_WORDS.bonesReady : ''}</span>
    {three && host !== null && build.accepted !== null && createPortal(<ThreeStage accepted={build.accepted} filmDriver={filmDriver} placing={placing} />, host)}
    {three && !placing && build.accepted !== null && <Measurements accepted={build.accepted} />}
    {three && placing && build.accepted !== null && <PlaceSheet accepted={build.accepted} />}
    <FilmCredit />
    <Stage sideControls={slug === 'films' && sideOrigin === 'real-case' ? <SideFacing key={sideOpening} opening={sideOpening} /> : reviewSide ? <SideReview onLooksRight={() => { findState.getState().approveSide(sideReviewKey(sideMarks, sideRevision)); }} onRename={renameSide} onLeaveOut={leaveSideOut} /> : search.sideExclusion === 'facing' ? <SideFacing key={sideOpening} opening={sideOpening} /> : search.sideExclusion !== null ? <p className="side-controls">{exclusionWords(search.sideExclusion)}</p> : null} compared={three && !placing ? threeView.compared : null} sheetOpen={three && (placing ? placeView.sheet : threeView.sheet)} {...(placing?{}:{onPressFilm:(kind:FilmKind) => { if (!three || threeView.moving) return; threeState.getState().compare(threeView.compared === kind ? null : kind); }})} forming={three && build.accepted?.movePending === true} onFilmDriver={setFilmDriver} moment={three ? 'three' : slug === 'find' || slug === 'three' && c.marks.length > 0 ? 'find' : 'films'} films={films} reading={intake.making ? FILM_ORDER : intake.reading} held={held} onPressPlace={pressPlace} leaving={intake.leaving} marked={finding.marked} reviewing={finding.reviewing} replay={finding.replay} {...(three ? { onPoint: (kind, key) => { const mark = c.marks.find((m) => m.film === kind)?.vertebrae.find((v) => String(v.id) === key); threeState.getState().point(mark?.level ?? null); } } : finding.onPoint === undefined ? {} : { onPoint: finding.onPoint })} />
  </>;
}
