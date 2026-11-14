import { TEMPLATE_LEVELS } from '@/shapes';
import { Component, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useStore } from 'zustand';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { currentStep } from '@/view-state/current-step';
import { threeState } from '@/view-state/three-state';
import { Spine3D, type Spine3DHandle } from '@/spine-3d/view/Spine3D';
import { restingView as fittedRestingView } from './resting-view';
import type { ViewName } from '@/spine-3d/looking/named-views';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { usePageState } from '@/look-and-feel/page-state';
import { comparisonState } from '../place/comparison-state';
import { ComparisonDrawing } from '../place/ComparisonDrawing';
import { PlaceDrawing } from '../place/PlaceDrawing';
import { placeState } from '../place/place-state';
import { openWhy } from '../place/hand-state';
import { PLACE_WORDS } from '@/words/place';
import { NameChip } from './NameChip';
import { compareFilm } from './comparison';
import { createMove, type Move } from './move';
import type { FilmDriver } from '../Stage';
import { failSpineView } from './build-spine';
import './ThreeStage.css';

class ViewBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  override state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  override componentDidCatch() { failSpineView(); }
  override render() { return this.state.failed ? null : this.props.children; }
}
function Drawing({ accepted, filmDriver, placing = false }: { accepted: AcceptedSpine; filmDriver?: FilmDriver | null; placing?: boolean }) {
  const comparing=useStore(comparisonState,(s)=>s.open);
  const arriving = useStore(placeState, (s) => s.arriving);
  const lastPart = useRef<string | null>(null);
  const placeSheet = useStore(placeState, (s) => s.sheet);
  const pointed = useStore(threeState, (s) => s.pointed);
  const rowLevels = useStore(threeState, (s) => s.rowLevels);
  const highlighted = useMemo(() => rowLevels.length > 0 ? rowLevels : pointed === null ? [] : [pointed], [pointed, rowLevels]);
  usePageState('links', highlighted.length > 0);
  usePageState('placement-comparison',placing&&comparing);
  const [pointer, setPointer] = useState({ x: 0, y: 0 });
  const root = useRef<HTMLDivElement>(null);
  const move = useRef<Move | null>(null);
  const compared = useStore(threeState, (s) => s.compared);
  const sheet = useStore(threeState, (s) => s.sheet);
  const previousCompare = useRef<typeof compared>(null);
  const origin = useRef<HTMLElement | null>(null);
  const closeComparison = () => { threeState.getState().compare(null); origin.current?.focus(); };
  useKey('review', ['Escape'], closeComparison, () => compared !== null);
  const moving = useStore(threeState, (s) => s.moving);
  const handle = useRef<Spine3DHandle>(null);
  const chosen = useStore(threeState, (s) => s.chosen);
  useLayoutEffect(() => { lastPart.current = null; }, [accepted]);
  useLayoutEffect(() => {
    if (placing && !arriving && lastPart.current !== null) placeState.setState({ hovered: lastPart.current });
  }, [placing, arriving]);
  const drawing = useMemo(() => ({ ...accepted.fit, vertebrae: accepted.fit.vertebrae.filter((v) => v.level !== 'C7' && TEMPLATE_LEVELS.standard.includes(v.level)) }), [accepted]);
  const films = useMemo(() => ({
    front: { geometry: accepted.input.front.geometry, size: accepted.input.front.size, pixels: null },
    ...(accepted.fit.sideFilm === null || accepted.input.side === null ? {} : { side: { geometry: accepted.input.side.geometry, size: accepted.input.side.size, pixels: null } }),
  }), [accepted]);
  const restingView = (name: ViewName = 'three-quarter') => {
    const stage = handle.current?.stage(), part = root.current;
    if (stage == null || part === null) return null;
    return fittedRestingView(stage, part, name);
  };
  const latestRest = useRef(restingView);
  useLayoutEffect(() => { latestRest.current = restingView; });
  useLayoutEffect(() => {
    // Film effects have run before the walls are hidden. The DOM viewers remain the pictures.
    const stage = handle.current?.stage();
    stage?.film('front')?.set({ strength: 0 });
    stage?.film('side')?.set({ strength: 0 });
    const view = latestRest.current(); if (view !== null) stage?.setView(view);
  }, [accepted]);
  useLayoutEffect(() => {
    const part = root.current;
    const stage = handle.current?.stage(), spine = handle.current?.spine();
    const canvas = part?.querySelector('canvas'), host = part?.closest<HTMLElement>('.spine');
    const stageElement = host?.parentElement?.querySelector<HTMLElement>('main.stage');
    if (!accepted.movePending || filmDriver == null || filmDriver.moment !== 'three' || stage == null || spine == null || canvas == null || host == null || stageElement == null) return;
    threeState.getState().move(true);
    const glow = host.closest('.app-frame')?.querySelector<HTMLElement>('.glow');
    const made = createMove({ stage, spine, films: filmDriver, canvas, host, stageElement, restView: () => latestRest.current() ?? stage.namedView('three-quarter'), ...(glow == null ? {} : { glow }), arrivalParts: () => [...stageElement.querySelectorAll<HTMLElement>('.panel, .panel h2, .panel .measures li, .panel-note, .panel-acts')], done: () => { accepted.movePending = false; threeState.getState().move(false); } });
    move.current = made; made.play();
    return () => { if (currentStep.getState().slug !== 'three') accepted.movePending = false; made.dispose(); move.current = null; threeState.getState().move(false); };
  }, [accepted, filmDriver]);
  useLayoutEffect(() => {
    const on = handle.current, canvas = root.current?.querySelector('canvas');
    if (placing || on === null || canvas == null || filmDriver == null || filmDriver.moment !== 'three' || moving) return;
    if (compared === null) {
      if (!accepted.movePending) on.lookFrom(latestRest.current(threeState.getState().chosen ?? 'three-quarter') ?? 'three-quarter', { animate: previousCompare.current !== null });
      previousCompare.current = null; return;
    }
    if (filmDriver.compared !== compared) return;
    previousCompare.current = compared;
    origin.current = document.querySelector<HTMLElement>(`.tile[data-place="${compared}"]`);
    return compareFilm(filmDriver, on, canvas);
  }, [accepted, compared, filmDriver, moving, placing]);
  useEffect(() => { if (!placing && chosen !== null) handle.current?.lookFrom(latestRest.current(chosen) ?? chosen); }, [chosen, placing]);
  return <div ref={root} className="three-stage" tabIndex={placing ? 0 : undefined} aria-label={placing ? PLACE_WORDS.stage : undefined} data-place={placing || undefined} data-testid="three-stage" data-state={moving ? 'moving' : 'settled'} data-compared={compared ?? undefined} inert={sheet || placing && placeSheet} onPointerMove={(event) => {
      if(placing){if(placeState.getState().keyboard)placeState.setState({keyboard:false});return;}
      if (moving || sheet) return;
      setPointer({ x: event.clientX, y: event.clientY });
      if (compared !== null) {
        const canvas = root.current?.querySelector('canvas'), stage = handle.current?.stage();
        if (canvas !== null && canvas !== undefined && stage !== null && stage !== undefined) {
          const box = canvas.getBoundingClientRect(); threeState.getState().point(stage.pick(event.clientX - box.left, event.clientY - box.top)?.level ?? null);
        }
      }
    }} onPointerLeave={() => { threeState.getState().point(null); if (placing) { lastPart.current = null; placeState.setState({ hovered: null }); } }} onPointerDown={() => { if (placing) root.current?.focus(); }}>
    <NameChip level={moving || sheet || placing ? null : pointed} x={pointer.x} y={pointer.y} />
    <Spine3D ref={handle} fit={drawing} bones={accepted.bones} films={films} view="three-quarter" highlighted={highlighted} onPoint={(level) => { if (!placing) threeState.getState().point(level); }} onPointed={(found) => { if (!placing||comparing) return; lastPart.current = found?.kind === 'part' ? found.id : null; if (!placeState.getState().arriving&&!placeState.getState().keyboard) placeState.setState({ hovered: lastPart.current }); }} onChosen={(found) => { if (placing && !comparing && !placeState.getState().arriving) {const id=found?.kind==='part'?found.id:null;if(id==='upper'||id==='lower')openWhy(id);else placeState.setState({pressed:id});} }} onChoose={(level) => { threeState.getState().point(level); }} turnable={!moving && compared === null && !sheet && (!placing || !placeSheet)} shadow onTurned={() => { if (placing) placeState.setState({ chosen: null }); threeState.getState().choose(null); threeState.getState().point(null); }} />
    {placing && (comparing?<ComparisonDrawing accepted={accepted} handle={handle}/>:<PlaceDrawing accepted={accepted} handle={handle}/>)}
  </div>;
}

export function ThreeStage({ accepted, filmDriver, placing = false }: { accepted: AcceptedSpine; filmDriver?: FilmDriver | null; placing?: boolean }) {
  return <ViewBoundary><Drawing accepted={accepted} placing={placing} {...(filmDriver === undefined ? {} : { filmDriver })} /></ViewBoundary>;
}
