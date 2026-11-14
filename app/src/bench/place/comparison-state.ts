import { createStore } from 'zustand/vanilla';
import type { BenchBatch, BenchResult, ImplantLayout } from '@/shapes';
import { EngineFailure,type EngineClient } from '@/engine-room/engine-client';
import { currentStep } from '@/view-state/current-step';
import { placeState } from './place-state';
import { handState } from './hand-state';
import { tooFewSettled } from './load-view';
import { loadState } from './load-state';
import { comparisonLayouts,comparisonMeasures, type ComparisonMeasure } from './comparison-view';
interface ComparisonState { open:boolean; leaving:boolean; phase:'idle'|'running'|'done'|'failed'; layouts:ImplantLayout[]; finished:ImplantLayout[]; batches:BenchBatch[]; result:BenchResult|null; measure:ComparisonMeasure; page:number;pageSize:1|3; elapsedMs:number|null; retryable:boolean }
const initial:ComparisonState={open:false,leaving:false,phase:'idle',layouts:[],finished:[],batches:[],result:null,measure:'main-cobb',page:0,pageSize:3,elapsedMs:null,retryable:false};
export const comparisonState=createStore<ComparisonState>(()=>initial);
let generation=0;
placeState.subscribe((state,before)=>{if(state.key!==before.key||state.result!==before.result){generation++;comparisonState.setState(initial,true);}});
export function stopComparison():void {generation++;const state=comparisonState.getState();if(state.phase==='running')comparisonState.setState({phase:'done',layouts:state.finished,page:0});}
/** Done is not Stop: unfinished and refused comparisons must be worked out afresh. */
export function closeComparison(immediate=false):void {
  const state=comparisonState.getState();if(!state.open)return;
  generation++;const mine=generation,keep=state.phase==='done';
  const finish=()=>{if(mine!==generation)return;if(keep)comparisonState.setState({open:false,leaving:false});else comparisonState.setState(initial,true);};
  if(immediate){finish();return;}
  comparisonState.setState({leaving:true});setTimeout(finish,640);
}
currentStep.subscribe((state)=>{if(state.slug!=='place')closeComparison(true);});
/** Each completed placement has exactly the baseline's versions, including its zero slice.
 * The Loaded result is never replaced: comparison scales and ink belong to the columns only. */
export async function comparePlacements(engine:Pick<EngineClient,'runLoadBench'|'compareLayouts'>):Promise<void> {
  if(loadState.getState().phase!=='loaded'||tooFewSettled(loadState.getState().result))return;
  const cached=comparisonState.getState();
  if(cached.phase==='done'){comparisonState.setState({open:true,leaving:false});return;}
  if(cached.phase==='running')return;
  const state=placeState.getState(),load=loadState.getState(),proposals=handState.getState().first??state.result,baseline=state.result?.layouts[0],input=state.input;
  if(baseline===undefined||proposals===null||input===null||load.result===null||load.phase!=='loaded')return;
  const layouts=comparisonLayouts(baseline,proposals),finished=[baseline],batches=[...load.batches],mine=++generation,started=performance.now();
  comparisonState.setState({...initial,open:true,phase:'running',measure:comparisonMeasures(load.result)[0]??'heaviest-anchor-demand',layouts,finished:[...finished],batches:[...batches],result:load.result});
  placeState.setState({review:null,sheet:false,pressed:null,hovered:null});
  try {
    for(const layout of layouts.slice(1)) {
      const slices:BenchBatch[]=[];
      for(let from=0;from<input.versions;from+=4){
        const batch=await engine.runLoadBench({fit:input.fit,layouts:[layout],child:input.child,seed:input.seed,versions:input.versions,versionFrom:from,versionCount:Math.min(4,input.versions-from)});
        if(mine!==generation)return;slices.push(batch);
      }
      const next=[...finished,layout],all=[...batches,...slices],result=await engine.compareLayouts({layouts:next,batches:all});
      if(mine!==generation)return;
      finished.push(layout);batches.push(...slices);
      comparisonState.setState({finished:[...finished],batches:[...batches],result});
    }
    comparisonState.setState({phase:'done',elapsedMs:performance.now()-started});
  } catch(error) {if(mine!==generation)return;const retryable=error instanceof EngineFailure&&['engine-crashed','engine-out-of-memory','engine-load-failed'].includes(error.error.code);if(retryable)comparisonState.setState({phase:'failed',retryable,layouts:[...finished],elapsedMs:performance.now()-started});else comparisonState.setState(initial,true);}
}
