import { createStore } from 'zustand/vanilla';
import type { CaseFileV1, LayoutProposals } from '@/shapes';
import { caseStore } from '@/case-state/case-store';
import { NO_HISTORY } from '@/case-state/undo-history';
import type { EngineClient } from '@/engine-room/engine-client';
import { currentStep } from '@/view-state/current-step';
import { spineBuild, type AcceptedSpine } from '@/view-state/spine-build';
import { layoutInput } from './layout-input';
import { placementFailure, type PlacementFailure } from './placement-failure';
export type PlaceView = 'front' | 'side' | 'top' | 'back';
export interface PlaceState {
  key: string | null; fitKey: string | null; phase: 'idle' | 'working' | 'ready' | 'failed'; result: LayoutProposals | null; failure: PlacementFailure | null;
  hovered: string | null; pressed: string | null; changed: boolean; arriving: boolean; review: number | null; sheet: boolean; chosen: PlaceView | null;
}
const initial: PlaceState = { key: null, fitKey: null, phase: 'idle', result: null, failure: null, hovered: null, pressed: null, changed: false, arriving: false, review: null, sheet: false, chosen: 'back' };
export const placeState = createStore<PlaceState>()(() => initial);
let shownFit: string | null = null, request = 0;
/** Only a placement actually shown can be replaced by a changed spine. */
export function notePlacementShown(): void {
  const state = placeState.getState();
  if (state.phase === 'ready' && !state.arriving && (state.result?.layouts.length ?? 0) > 0) {
    const changed = shownFit !== null && shownFit !== state.fitKey;
    shownFit = state.fitKey;
    if (changed) placeState.setState({ changed: true });
  }
}
currentStep.subscribe((state) => {
  if (state.slug !== 'place') placeState.setState({ sheet: false, review: null, hovered: null, pressed: null, changed: false });
});
caseStore.subscribe((state, before) => {
  // Opening resets the history to this sentinel. Starting the history afresh
  // keeps the same case object; undoing an edit keeps its own history object.
  if (state.case !== before.case && (state.history === NO_HISTORY || state.case.films.length === 0)) {
    shownFit = null; request++; placeState.setState(initial, true);
  }
});
spineBuild.subscribe((state) => {
  if (state.accepted === null || state.accepted.key !== placeState.getState().fitKey) { request++; placeState.setState(initial, true); }
});
export async function requestPlacement(engine: Pick<EngineClient, 'proposeLayouts'>, accepted: AcceptedSpine, c: CaseFileV1, retry = false): Promise<void> {
  const input = layoutInput(accepted.fit, c);
  const key = JSON.stringify({ fitKey: accepted.key, input });
  if (!retry && placeState.getState().key === key) return;
  const changed = shownFit !== null && shownFit !== accepted.key;
  const mine = ++request;
  placeState.setState({ ...initial, key, fitKey: accepted.key, phase: 'working', changed });
  try {
    const result = await engine.proposeLayouts(input);
    if (mine !== request) return;
    placeState.setState({ result, phase: 'ready', arriving: result.layouts.length > 0 });
  } catch (error) {
    const failure = placementFailure(error);
    if (failure !== null && mine === request) placeState.setState({ phase: 'failed', failure });
  }
}
