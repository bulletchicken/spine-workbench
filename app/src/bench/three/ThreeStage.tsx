import { TEMPLATE_LEVELS } from '@/shapes';
import { Component, useEffect, useLayoutEffect, useMemo, useRef, type ReactNode } from 'react';
import { useStore } from 'zustand';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import { Spine3D, type Spine3DHandle } from '@/spine-3d/view/Spine3D';
import { failSpineView } from './build-spine';
import './ThreeStage.css';

class ViewBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  override state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  override componentDidCatch() { failSpineView(); }
  override render() { return this.state.failed ? null : this.props.children; }
}
function Drawing({ accepted }: { accepted: AcceptedSpine }) {
  const handle = useRef<Spine3DHandle>(null);
  const chosen = useStore(threeState, (s) => s.chosen);
  const drawing = useMemo(() => ({ ...accepted.fit, vertebrae: accepted.fit.vertebrae.filter((v) => v.level !== 'C7' && TEMPLATE_LEVELS.standard.includes(v.level)) }), [accepted]);
  const films = useMemo(() => ({
    front: { geometry: accepted.input.front.geometry, size: accepted.input.front.size, pixels: null },
    ...(accepted.fit.sideFilm === null || accepted.input.side === null ? {} : { side: { geometry: accepted.input.side.geometry, size: accepted.input.side.size, pixels: null } }),
  }), [accepted]);
  useLayoutEffect(() => {
    // Film effects have run before the walls are hidden. The DOM viewers remain the pictures.
    const stage = handle.current?.stage();
    stage?.film('front')?.set({ strength: 0 });
    stage?.film('side')?.set({ strength: 0 });
  }, [accepted]);
  useEffect(() => { if (chosen !== null) handle.current?.lookFrom(chosen); }, [chosen]);
  return <div className="three-stage" data-testid="three-stage" data-state="settled">
    <Spine3D ref={handle} fit={drawing} bones={accepted.bones} films={films} view="three-quarter" highlighted={[]} shadow onTurned={() => { threeState.getState().choose(null); }} />
  </div>;
}

export function ThreeStage({ accepted }: { accepted: AcceptedSpine }) {
  return <ViewBoundary><Drawing accepted={accepted} /></ViewBoundary>;
}
