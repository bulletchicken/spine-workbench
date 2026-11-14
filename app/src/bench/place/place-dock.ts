import type { BenchStop, ImageRight } from '@/shapes';
import type { DockSpec } from '@/guide/dock-spec';
import { ESTIMATE_TAGS, LOAD_STOPS, SHOWN_LOAD_STOPS, PLACE_LOAD_WORDS as L, PLACE_WORDS as W } from '@/words/place';
import { placeState, type PlaceState, type PlaceView } from './place-state';
import { backToPlacement, loadState } from './load-state';
import { rangeWords, stopResult, shownLoadStop, unsettledCount } from './load-view';
import { countWords, reviewItems } from './placement-view';
export interface PlaceActions { back: () => void; retry: () => void; review: (at: number | null) => void; sheet: () => void; view: (name: PlaceView) => void; load?: () => void; stop?: () => void; estimates?: () => void; standardNames?: boolean }
export function placeDock(state: PlaceState, actions: PlaceActions, namedFor: ImageRight = 'patient-left', shownStops: readonly BenchStop[] = SHOWN_LOAD_STOPS): DockSpec {
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
  const storedLoad=loadState.getState(),load={...storedLoad,stop:shownLoadStop(storedLoad.stop,shownStops)};
  if(load.phase==='running')return {...base,focus:'quiet',sentence:L.loading(load.total),helper:{kind:'count',count:W.progress(load.completed,load.total),spoken:W.progress(load.completed,load.total),share:load.total===0?0:load.completed/load.total},quiet:{label:L.stop,press:actions.stop??(()=>undefined)}};
  if(load.phase==='failed')return {...base,focus:load.failure?.kind==='failed'?'primary':'quiet',sentence:load.failure?.reason??L.refused,quiet:{label:L.backToPlacement,press:backToPlacement},...(load.failure?.missingLevels===true&&actions.standardNames===true?{helper:{kind:'words' as const,words:[L.addFilm]}}:{}),...(load.failure?.kind==='failed'&&actions.load!==undefined?{primary:{label:W.retry,press:actions.load}}:{})};
  const items = reviewItems(result, namedFor), item = state.review === null ? undefined : items[state.review];
  const more = { label: W.more, controls: 'place-sheet', expanded: state.sheet, press: actions.sheet };
  if (item !== undefined && state.review !== null) return { ...base, sentence: W.needsLook,
    review: { label: item.label, sentences: item.sentences, count: W.progress(state.review + 1, items.length), ...(item.details === undefined ? {} : { details: item.details }) },
    ...(state.review < items.length - 1 ? { primary: { label: W.next, press: () => { actions.review((state.review ?? 0) + 1); } } } : {}),
    quiet: { label: W.done, press: () => { actions.review(null); } } };
  if(load.phase==='loaded'&&load.result!==null){
    const unsettled=unsettledCount(load.result);
    const stop=stopResult(load.result,load.stop),curve=stop?.estimates.find((e)=>e.measure==='main-cobb'),hasCurve=load.result.mainCurve!==null||result.curves.some((c)=>c.versionsMajor>0),heaviest=stop?.estimates.find((e)=>e.measure==='heaviest-anchor-demand');
    const showAnchor=load.stop!=='before'&&(load.stop!=='rods-in'||!hasCurve),tag=load.stop==='before'?undefined:showAnchor?heaviest?.tag:hasCurve?curve?.tag:undefined;
    const helper=[load.stop==='before'?L.noLoad:showAnchor&&heaviest!==undefined?L.anchorHelper(rangeWords(heaviest.measure,heaviest.band)):'',unsettled>0?L.unsettled(unsettled,load.result.versions):''].filter(Boolean).join(' ');
    return {...base,focus:'chosen-stop',sentence:load.stop==='before'?L.before:L.loaded(LOAD_STOPS[load.stop],hasCurve&&curve!==undefined?rangeWords('main-cobb',curve.band):null),
      ...(helper===''?{}:{helper:{kind:'words' as const,words:[helper]}}),
      ...(tag===undefined?{}:{small:ESTIMATE_TAGS[tag]}),
      stops:{chosen:load.stop,choose:(stop)=>{loadState.setState({stop});placeState.setState({pressed:null,hovered:null});}},quiet:{label:L.allEstimates,press:actions.estimates??actions.sheet}};
  }
  return { ...base, focus:'primary', sentence: W.placed(layout.upper, layout.lower), helper: { kind: 'words', words: [[result.firstIsSketch ? W.sketch : '', countWords(result), load.phase==='stopped'?L.stopped:'', state.changed ? W.changed : ''].filter(Boolean).join(' ')] },
    ...(actions.load===undefined?{}:{primary:{label:L.load,press:actions.load}}),
    more, views: { chosen: state.chosen, choose: actions.view, back: true },
    ...(items.length > 0 ? { quiet: { label: W.review(items.length), press: () => { actions.review(0); } } } : {}) };
}
