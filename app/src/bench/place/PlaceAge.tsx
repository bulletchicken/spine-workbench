import { useStore } from 'zustand';
import { useCase } from '@/case-state/case-store';
import { useEngine } from '@/engine-room/use-engine';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { PLACE_AGE_WORDS as W } from '@/words/place';
import { placeState,changeAge } from './place-state';
import { loadState } from './load-state';
export function PlaceAge({accepted}:{accepted:AcceptedSpine}) {
  const age=useStore(placeState,(s)=>s.ageYears),running=useStore(loadState,(s)=>s.phase==='running'),c=useCase((s)=>s.case),engine=useEngine();
  return <div className="place-age"><label htmlFor="place-age">{W.age}</label><select id="place-age" value={age??''} disabled={running} onChange={(event)=>{if(loadState.getState().phase!=='running')void changeAge(engine,accepted,c,event.target.value===''?null:Number(event.target.value));}}><option value="">{W.unknown}</option>{Array.from({length:14},(_,i)=>i+5).map((value)=><option key={value} value={value}>{value}</option>)}</select></div>;
}
