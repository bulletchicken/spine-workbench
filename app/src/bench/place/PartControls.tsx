import { useStore } from 'zustand';
import type { AnchorProposal, ImplantLayout, LayoutChoices, RodProposal } from '@/shapes';
import { Button } from '@/building-blocks/Button';
import { Segmented } from '@/building-blocks/Segmented';
import { useEngine } from '@/engine-room/use-engine';
import { PLACE_EDIT_WORDS as W, PLACE_WORDS } from '@/words/place';
import { changePlacement, choiceReason, handState } from './hand-state';
import { loadState } from './load-state';
function Stepper({label,value,grid,less,more,change,busy}:{label:string;value:number|null;grid:readonly number[];less:string;more:string;change:(n:number)=>void;busy:boolean}) {
  if(value===null||grid.length===0)return null;
  const at=grid.indexOf(value),previous=grid.filter((n)=>n<value).at(-1),next=grid.find((n)=>n>value);
  return <div className="place-stepper"><span>{label}</span><Button look="text" disabled={previous===undefined} busy={busy} aria-label={less} onPress={()=>{if(previous!==undefined)change(previous);}}>{W.less}</Button><span className="num">{label===W.diameter?PLACE_WORDS.diameter(value):W.lengthSize(value)}</span><Button look="text" disabled={next===undefined||at===grid.length-1} busy={busy} aria-label={more} onPress={()=>{if(next!==undefined)change(next);}}>{W.more}</Button></div>;
}
export function PartControls({layout,anchor,rod,choices}:{layout:ImplantLayout;anchor:AnchorProposal|undefined;rod:RodProposal|undefined;choices:LayoutChoices}) {
  const engine=useEngine(),hand=useStore(handState),running=useStore(loadState,(s)=>s.phase==='running'),busy=hand.busy!==null||running;
  if(anchor!==undefined){
    const set=(kind:AnchorProposal['kind'],diameterMm:number|null=null,lengthMm:number|null=null)=>{if(!busy)void changePlacement(engine,{kind:'anchor',level:anchor.level,side:anchor.side,anchorKind:kind,diameterMm,lengthMm});};
    const kinds=[['screw',W.screw],['hook',W.hook],['none',W.none]] as const;
    return <div className="place-part-controls" aria-busy={busy||undefined}>
      <Segmented className="part-kinds" label={W.kind} busy={busy} chosen={anchor.kind} options={kinds.map(([id,label])=>({id,label,disabled:choiceReason(layout,anchor,id)!==null}))} onChoose={(kind)=>{if(kind!==anchor.kind&&(kind==='screw'||kind==='hook'||kind==='none'))set(kind);}}/>
      {[...new Set(kinds.map(([kind])=>choiceReason(layout,anchor,kind)).filter((reason)=>reason!==null))].map((reason)=><p key={reason}>{reason}</p>)}
      {anchor.kind==='screw'&&<><Stepper label={W.diameter} value={anchor.diameterMm} grid={choices.screwDiametersMm} less={W.decreaseDiameter} more={W.increaseDiameter} busy={busy} change={(diameter)=>{set('screw',diameter,anchor.lengthMm);}}/><Stepper label={W.length} value={anchor.lengthMm} grid={choices.screwLengthsMm} less={W.decreaseLength} more={W.increaseLength} busy={busy} change={(length)=>{set('screw',anchor.diameterMm,length);}}/></>}
      {hand.failure!==null&&<p role="status">{W.failed}{hand.failure==='failed'&&hand.retry!==null&&<Button look="text" onPress={()=>{if(hand.retry!==null)void changePlacement(engine,hand.retry);}}>{PLACE_WORDS.retry}</Button>}</p>}
    </div>;
  }
  if(rod!==undefined)return <div className="place-part-controls" aria-busy={busy||undefined}><p>{W.rodPair}</p><div className="place-rod-choices">{choices.rods.map((pair)=><Button key={`${pair.material}:${String(pair.diameterMm)}`} look="text" busy={busy} aria-pressed={rod.material===pair.material&&rod.diameterMm===pair.diameterMm} onPress={()=>{if(rod.material!==pair.material||rod.diameterMm!==pair.diameterMm)void changePlacement(engine,{kind:'rod',pair});}}>{W.rodOption(W.materials[pair.material],pair.diameterMm)}</Button>)}</div>{hand.failure!==null&&<p role="status">{W.failed}</p>}</div>;
  return null;
}
