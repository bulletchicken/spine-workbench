import { emptyCase } from '@/case-state/empty-case';
import { layoutInput } from './layout-input';
import { useEngine } from '@/engine-room/use-engine';
import { changePlacement, handState, undoPlacement } from './hand-state';
import { useLayoutEffect, useMemo } from 'react';
import { useStore } from 'zustand';
import type { GalleryPage } from '@/gallery/gallery-page';
import { Dock } from '@/guide/Dock';
import { MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import { useBones } from '@/spine-3d/view/gallery-parts';
import { PLACE_WORDS as W } from '@/words/place';
import { ThreeStage } from '../three/ThreeStage';
import { placeState } from './place-state';
import { placeDock } from './place-dock';
import { placementFixture } from './placement-fixture.scenes';
import { PlaceSheet } from './PlaceSheet';
import './Place.gallery.css';
function PlaceGallery() {
  const engine=useEngine();useStore(handState);
  const bones = useBones(), state = useStore(placeState), result = useMemo(() => placementFixture(), []);
  useLayoutEffect(() => {
    const before = placeState.getState();
    placeState.setState({ key: 'gallery-place',input:layoutInput(MADE_UP_CASE.fit,emptyCase()), phase: 'ready', result, arriving: true, review: null, sheet: false, chosen: 'back' });
    return () => { placeState.setState(before, true); };
  }, [result]);
  const accepted = useMemo(() => ({ key: 'gallery-place', fit: MADE_UP_CASE.fit, bones: bones ?? [], input: {
    front: { marks: { film: 'front' as const, vertebrae: [], landmarks: { c7: null, sacrumTop: null, hips: [] }, naming: { template: 'standard' as const, chosenBy: 'engine' as const, alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: null }, namedFor: 'patient-left' as const }, geometry: MADE_UP_CASE.front.geometry, size: MADE_UP_CASE.front.size, markSpread: MADE_UP_CASE.front.markSpread, facingAssumed: true }, side: null, typical: 'built-in' as const, twistReadings: [] } }), [bones]);
  return <div className="place-gallery" data-step="place"><div className="place-gallery-stage stage">{bones !== null && <ThreeStage accepted={accepted} placing />}<PlaceSheet accepted={accepted} /></div><Dock spec={placeDock(state, { change:(change)=>{void changePlacement(engine,change);},undo:undoPlacement,back: () => undefined, retry: () => undefined, review: (review) => { placeState.setState({ review, sheet: false }); }, sheet: () => { placeState.setState({ sheet: !state.sheet }); }, view: (chosen) => { placeState.setState({ chosen }); } })} /></div>;
}
const page: GalleryPage = { area: 'bench', part: W.gallery, states: [{ name: W.galleryState, show: () => <PlaceGallery /> }] };
export default page;
