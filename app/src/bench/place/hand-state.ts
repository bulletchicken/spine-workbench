import { createStore } from 'zustand/vanilla';
import { TEMPLATE_LEVELS, type AnchorChoice, type AnchorKind, type AnchorProposal, type ImplantLayout, type LayoutProposals, type LayoutInput, type Level, type RebuildInput, type RodChoice } from '@/shapes';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { PLACE_EDIT_WORDS as W } from '@/words/place';
import { placeState } from './place-state';
export type End = 'upper' | 'lower';
export type HandChange = { kind:'anchor'; level:Level; side:AnchorChoice['side']; anchorKind:AnchorKind; diameterMm:number|null; lengthMm:number|null } | {kind:'end';end:End;level:Level} | {kind:'rod';pair:RodChoice};
interface Snapshot { result:LayoutProposals; choices:AnchorChoice[] }
interface HandState { first:LayoutProposals|null; past:Snapshot[]; choices:AnchorChoice[]; busy:string|null; failure:'failed'|'refused'|null; retry:HandChange|null; why:End|null; drag:{end:End;level:Level}|null; flashing:string[] }
const initial:HandState={first:null,past:[],choices:[],busy:null,failure:null,retry:null,why:null,drag:null,flashing:[]};
export const handState=createStore<HandState>(()=>initial);
let generation=0;
placeState.subscribe((state,before)=>{
  if(state.key!==before.key){generation++;handState.setState(initial,true);}
  else if(before.sheet&&!state.sheet)handState.setState({why:null});
});
/** The film's declared count wins; fit-only input follows the engine's T13 inference. */
export function levelOrder(input:LayoutInput|null=placeState.getState().input):readonly Level[] {
  const template=input?.front?.marks.naming.template??(input?.fit.vertebrae.some((v)=>v.level==='T13')===true?'extra-thoracic':'standard');
  const levels=TEMPLATE_LEVELS[template],last=levels.indexOf('L5');
  return levels.slice(levels.indexOf('T1'),last<0?undefined:last+1);
}
export function fittedLevels(input:LayoutInput|null=placeState.getState().input):Level[] {return levelOrder(input).filter((l)=>input?.fit.vertebrae.some((v)=>v.level===l)===true);}
export function cancelEndDrag():void {handState.setState({drag:null});}
export function choiceReason(layout:ImplantLayout,anchor:AnchorProposal,kind:AnchorKind):string|null {
  if(kind==='screw'&&anchor.screwHere===null)return W.noScrew;
  if((kind==='hook'||kind==='screw')&&anchor.directionStatus==='missing')return W.noDirection;
  if(kind==='none'&&anchor.kind!=='none'&&layout.anchors.filter((a)=>a.side===anchor.side&&a.kind!=='none').length<=2)return W.twoAnchors;
  return null;
}
export function endLevels(layout:ImplantLayout,levels:readonly Level[],end:End):Level[] {return levels.filter((level)=>endReason(layout,end,level,levels)===null);}
export function endReason(layout:ImplantLayout,end:End,level:Level,levels:readonly Level[]=fittedLevels()):string|null {
  const order=levelOrder(),at=order.indexOf(level),other=order.indexOf(end==='upper'?layout.lower:layout.upper);
  if(at<0||other<0||!levels.includes(level))return W.fittedSpan;
  if(end==='upper'?at>=other:at<=other)return W.endsApart;
  const low=end==='upper'?at:order.indexOf(layout.upper),high=end==='lower'?at:order.indexOf(layout.lower);
  if(!order.slice(low,high+1).every((l)=>levels.includes(l)))return W.fittedSpan;
  if(['left','right'].some((side)=>layout.anchors.filter((a)=>{const i=order.indexOf(a.level);return a.side===side&&a.kind!=='none'&&i>=low&&i<=high;}).length<2))return W.twoAnchors;
  return null;
}
export function snapEnd(layout:ImplantLayout,end:End,wanted:Level,levels:readonly Level[]):Level {
  const order=levelOrder(),legal=endLevels(layout,levels,end);
  return legal.reduce((best,l)=>Math.abs(order.indexOf(l)-order.indexOf(wanted))<Math.abs(order.indexOf(best)-order.indexOf(wanted))?l:best,layout[end]);
}
export function rebuildInput(change:HandChange,choices:readonly AnchorChoice[]=handState.getState().choices):{input:RebuildInput;choices:AnchorChoice[]}|null {
  const state=placeState.getState(),layout=state.result?.layouts[0],rod=layout?.rods[0];if(state.input===null||layout===undefined||rod===undefined)return null;
  const next=choices.map((a)=>({...a}));
  if(change.kind==='anchor'){const at=next.findIndex((a)=>a.level===change.level&&a.side===change.side),choice:AnchorChoice={level:change.level,side:change.side,kind:change.anchorKind,diameterMm:change.diameterMm,lengthMm:change.lengthMm};if(at<0)next.push(choice);else next[at]=choice;}
  const upper=change.kind==='end'&&change.end==='upper'?change.level:layout.upper,lower=change.kind==='end'&&change.end==='lower'?change.level:layout.lower;
  const pair=change.kind==='rod'?change.pair:{material:rod.material,diameterMm:rod.diameterMm},order=levelOrder(state.input),low=order.indexOf(upper),high=order.indexOf(lower);
  if(low<0||high<=low||!order.slice(low,high+1).every((l)=>fittedLevels(state.input).includes(l)))return null;
  const anchors=next.filter((a)=>{const i=order.indexOf(a.level);return i>=low&&i<=high;});
  if(change.kind==='anchor'&&!anchors.some((a)=>a.level===change.level&&a.side===change.side))return null;
  return {input:{layout:state.input,upper,lower,anchors,rodMaterial:pair.material,rodDiameterMm:pair.diameterMm},choices:anchors};
}
export function changedParts(before:ImplantLayout,after:ImplantLayout):string[] {
  const shape=(a:AnchorProposal)=>JSON.stringify([a.kind,a.diameterMm,a.lengthMm,a.entryMm,a.directionUnit]);
  return [...after.anchors.flatMap((a,i)=>{const old=before.anchors.find((b)=>b.level===a.level&&b.side===a.side);return old===undefined||shape(old)!==shape(a)?[`anchor:${String(i)}`]:[];}),...(['upper','lower'] as const).filter((end)=>before[end]!==after[end]),...after.rods.flatMap((r)=>JSON.stringify(r)!==JSON.stringify(before.rods.find((b)=>b.side===r.side))?[`rod:${r.side}`]:[])];
}
export async function changePlacement(engine:Pick<EngineClient,'rebuildLayout'>,change:HandChange):Promise<void> {
  const state=placeState.getState(),before=state.result,layout=before?.layouts[0],hand=handState.getState();if(hand.busy!==null||before==null||layout===undefined)return;
  if(change.kind==='anchor'){const a=layout.anchors.find((a)=>a.level===change.level&&a.side===change.side);if(a===undefined||choiceReason(layout,a,change.anchorKind)!==null)return;}
  if(change.kind==='end'&&(endReason(layout,change.end,change.level)!==null||!endLevels(layout,fittedLevels(),change.end).includes(change.level)))return;
  if(change.kind==='rod'&&!before.choices.rods.some((p)=>p.material===change.pair.material&&p.diameterMm===change.pair.diameterMm))return;
  const built=rebuildInput(change);if(built===null)return;
  const mine=++generation,busy=change.kind==='anchor'?`${change.level}:${change.side}`:change.kind==='rod'?'rod':change.end;
  handState.setState({busy,failure:null,retry:change});
  try{
    const answer=await engine.rebuildLayout(built.input);if(mine!==generation)return;
    const result={...before,layouts:[answer],firstIsSketch:false,namingScenarios:[],cannotTell:before.cannotTell.filter((r)=>r.code!=='ends-unsettled')};
    handState.setState({first:hand.first??before,past:[...hand.past,{result:before,choices:hand.choices}],choices:built.choices,busy:null,failure:null,retry:null,flashing:changedParts(layout,answer),why:null});
    const latest=placeState.getState();
    const remap=(id:string|null):string|null=>{
      if(id?.startsWith('anchor:')!==true)return id;
      const old=layout.anchors[Number(id.slice(7))];if(old===undefined)return null;
      const at=answer.anchors.findIndex((a)=>a.level===old.level&&a.side===old.side);
      return at<0?null:`anchor:${String(at)}`;
    };
    placeState.setState({result,review:null,pressed:remap(latest.pressed),hovered:remap(latest.hovered)});
  }catch(error){if(mine!==generation)return;const cancelled=error instanceof EngineFailure&&error.error.code==='pass-cancelled',temporary=error instanceof EngineFailure&&['engine-crashed','engine-out-of-memory','engine-load-failed'].includes(error.error.code);handState.setState({busy:null,failure:cancelled?null:temporary?'failed':'refused'});}
}
export function undoPlacement():void {
  const hand=handState.getState(),previous=hand.past.at(-1),current=placeState.getState().result?.layouts[0];if(previous===undefined||hand.busy!==null)return;
  generation++;handState.setState({past:hand.past.slice(0,-1),choices:previous.choices,failure:null,retry:null,flashing:current===undefined?[]:changedParts(current,previous.result.layouts[0]??current),why:null});
  placeState.setState({result:previous.result,review:null,pressed:null,hovered:null,keyboard:false});
}
export function startFirstPlacement():void {
  const hand=handState.getState(),first=hand.first,current=placeState.getState().result?.layouts[0];if(hand.busy!==null||hand.past.length===0||first===null)return;
  generation++;handState.setState({past:[],choices:[],failure:null,retry:null,why:null,flashing:current===undefined?[]:changedParts(current,first.layouts[0]??current)});
  placeState.setState({result:first,review:null,pressed:null,hovered:null,keyboard:false});
}
export function openWhy(end:End):void {handState.setState({why:end});placeState.setState({sheet:true,pressed:null,hovered:null});}
