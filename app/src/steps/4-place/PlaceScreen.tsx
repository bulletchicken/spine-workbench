import { changePlacement, handState, undoPlacement } from '@/bench/place/hand-state';
import { useEffect } from 'react';
import { useStore } from 'zustand';
import { hasCurrentFit } from '@/bench/three/build-spine';
import { loadPlacement, loadState, stopLoading } from '@/bench/place/load-state';
import { placeDock } from '@/bench/place/place-dock';
import { notePlacementShown, placeState, requestPlacement } from '@/bench/place/place-state';
import { useCase } from '@/case-state/case-store';
import { useEngine } from '@/engine-room/use-engine';
import { currentStep } from '@/view-state/current-step';
import { useShowInDock } from '@/view-state/dock-now';
import { spineBuild } from '@/view-state/spine-build';
const SPINE_CHANGED_NOTICE_MS = 6000;
export function PlaceScreen() {
  const c = useCase((s) => s.case), accepted = useStore(spineBuild, (s) => s.accepted), state = useStore(placeState), engine = useEngine();
  useStore(loadState);useStore(handState);
  const valid = hasCurrentFit(c);
  useEffect(() => { if (valid && accepted !== null) void requestPlacement(engine, accepted, c); }, [engine, accepted, c, valid]);
  useEffect(() => {
    if (state.phase !== 'ready' || state.arriving || (state.result?.layouts.length ?? 0) === 0) return;
    notePlacementShown();
    if (!state.changed) return;
    const timer = setTimeout(() => { placeState.setState({ changed: false }); }, SPINE_CHANGED_NOTICE_MS);
    return () => { clearTimeout(timer); };
  }, [state.changed, state.phase, state.arriving, state.result]);
  useShowInDock(placeDock(state, {
    change: (change) => {void changePlacement(engine,change);},
    undo: undoPlacement,
    load: () => { if(state.input!==null)void loadPlacement(engine,state.input); },
    stop: stopLoading,
    standardNames: c.marks.find((m)=>m.film==='front')?.naming.template==='standard',
    estimates: () => { handState.setState({why:null});placeState.setState({sheet:true}); requestAnimationFrame(()=>{const title=document.getElementById('all-estimates-title');title?.scrollIntoView({block:'start'});title?.focus({preventScroll:true});}); },
    back: () => { currentStep.getState().go('three'); },
    retry: () => { if (accepted !== null) void requestPlacement(engine, accepted, c, true); },
    review: (review) => { placeState.setState({ review, sheet: false, pressed: null, hovered: null }); },
    sheet: () => {handState.setState({why:null}); placeState.setState({ sheet: !state.sheet }); },
    view: (chosen) => { placeState.setState({ chosen }); },
  }, accepted?.input.front.marks.namedFor));
  return null;
}
