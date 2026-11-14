import type { ImplantLayout, LayoutProposals, Measure, PairedCount } from '@/shapes';
import { PLACE_COMPARE_WORDS as W } from '@/words/place';
import { levelOrder } from './hand-state';
export const comparisonPageSize=(width:number):1|3=>width<1280?1:3;
export const COMPARISON_MEASURES = ['main-cobb','heaviest-anchor-demand'] as const satisfies readonly Measure[];
export type ComparisonMeasure = typeof COMPARISON_MEASURES[number];
/** Identity is the actual hardware, not its id, evidence, or provenance. */
export function hardwareKey(layout:ImplantLayout):string {
  return JSON.stringify([layout.upper,layout.lower,layout.anchors.map((a)=>[a.level,a.side,a.kind,a.diameterMm,a.lengthMm,a.entryMm,a.directionUnit]),layout.rods.map((r)=>[r.side,r.material,r.diameterMm,r.pointsMm,r.overbendDeg,r.headOffsetMm,r.coronalLine]),layout.releasedLevels]);
}
export function comparisonLayouts(baseline:ImplantLayout,proposals:LayoutProposals):ImplantLayout[] {
  const seen=new Set([hardwareKey(baseline)]);
  return [baseline,...proposals.layouts.filter((layout)=>{const key=hardwareKey(layout);if(seen.has(key))return false;seen.add(key);return true;})];
}
export function comparisonName(layout:ImplantLayout,first:ImplantLayout):string {
  const change=layout.change,order=levelOrder();
  if(change.kind==='starts-at')return order.indexOf(change.level)<order.indexOf(first.upper)?W.startsHigher:W.startsLower;
  if(change.kind==='ends-at')return order.indexOf(change.level)<order.indexOf(first.lower)?W.endsHigher:W.endsLower;
  return W.names[change.kind];
}
export function pairedWords(count:PairedCount|undefined):{verdict:string;clear:string;dots:PairedCount['outcomes']} {
  if(count===undefined||count.order.length===0)return {verdict:W.notMeasured,clear:'',dots:[]};
  return {verdict:count.lower>=count.clearAt?W.lower(count.lower,count.order.length):count.higher>=count.clearAt?W.higher(count.higher,count.order.length):W.noClear,clear:W.clear(count.clearAt),dots:count.outcomes};
}

export function comparisonDifferences(baseline:ImplantLayout,layout:ImplantLayout):string[] {
  const anchorKey=(a:ImplantLayout['anchors'][number])=>JSON.stringify([a.kind,a.diameterMm,a.lengthMm,a.entryMm,a.directionUnit]);
  const rodKey=(r:ImplantLayout['rods'][number])=>JSON.stringify([r.material,r.diameterMm,r.pointsMm,r.overbendDeg,r.headOffsetMm,r.coronalLine]);
  return [...layout.anchors.flatMap((a,i)=>{const old=baseline.anchors.find((b)=>b.level===a.level&&b.side===a.side);return old===undefined||anchorKey(old)!==anchorKey(a)?[`anchor:${String(i)}`]:[];}),...layout.rods.flatMap((r)=>{const old=baseline.rods.find((b)=>b.side===r.side);return old===undefined||rodKey(old)!==rodKey(r)?[`rod:${r.side}`]:[];}),...(['upper','lower'] as const).filter((end)=>baseline[end]!==layout[end])];
}

export function comparisonMeasures(result:import('@/shapes').BenchResult|null):readonly ComparisonMeasure[] {return result?.mainCurve!=null?COMPARISON_MEASURES:['heaviest-anchor-demand'];}
