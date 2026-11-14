import type { ImageRight } from '@/shapes';
import type { DockSpec } from '@/guide/dock-spec';
import { PLACE_WORDS as W } from '@/words/place';
import type { PlaceState, PlaceView } from './place-state';
import { countWords, reviewItems } from './placement-view';
export interface PlaceActions { back: () => void; retry: () => void; review: (at: number | null) => void; sheet: () => void; view: (name: PlaceView) => void }
export function placeDock(state: PlaceState, actions: PlaceActions, namedFor: ImageRight = 'patient-left'): DockSpec {
  const base = { scene: 'place' as const };
  if (state.phase === 'idle' || state.phase === 'working' || state.arriving) return { ...base, sentence: W.working };
  if (state.phase === 'failed') {
    const failure = state.failure;
    return { ...base, sentence: failure?.reason ?? W.refused,
      ...(failure?.kind === 'failed' ? { primary: { label: W.retry, press: actions.retry } } : { quiet: { label: W.backToThree, press: actions.back } }) };
  }
  const result = state.result, layout = result?.layouts[0];
  if (result === null || layout === undefined) return { ...base, sentence: W.noPlacement,
    ...(result?.curves.length === 0 ? { helper: { kind: 'words' as const, words: [W.noCurve] } } : {}),
    more: { label: W.more, controls: 'place-sheet', expanded: state.sheet, press: actions.sheet }, quiet: { label: W.backToThree, press: actions.back } };
  const items = reviewItems(result, namedFor), item = state.review === null ? undefined : items[state.review];
  const more = { label: W.more, controls: 'place-sheet', expanded: state.sheet, press: actions.sheet };
  if (item !== undefined && state.review !== null) return { ...base, sentence: W.needsLook,
    review: { label: item.label, sentences: item.sentences, count: W.progress(state.review + 1, items.length), ...(item.details === undefined ? {} : { details: item.details }) },
    ...(state.review < items.length - 1 ? { primary: { label: W.next, press: () => { actions.review((state.review ?? 0) + 1); } } } : {}),
    quiet: { label: W.done, press: () => { actions.review(null); } } };
  return { ...base, sentence: W.placed(layout.upper, layout.lower), helper: { kind: 'words', words: [[result.firstIsSketch ? W.sketch : '', countWords(result), state.changed ? W.changed : ''].filter(Boolean).join(' ')] },
    more, views: { chosen: state.chosen, choose: actions.view, back: true },
    ...(items.length > 0 ? { quiet: { label: W.review(items.length), press: () => { actions.review(0); } } } : {}) };
}
