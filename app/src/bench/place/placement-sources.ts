import type { AnchorProposal,BenchResult,LayoutProposals,Level } from '@/shapes';
import { EVIDENCE_LEDGER,PLACE_SOURCES_WITHOUT_ENTRY,type EvidenceEntry } from '@/words/evidence-ledger';
import { PLACE_AGE_WORDS as A,PLACE_WORDS as W } from '@/words/place';
export type PlaceEvidence=EvidenceEntry;
export type SourceFamily='layout'|'bench';
export interface SourceUse {key:string;from:SourceFamily}
export type SourceKind=typeof PLACE_SOURCES_WITHOUT_ENTRY[keyof typeof PLACE_SOURCES_WITHOUT_ENTRY];
export interface SourceRecord {key:string;from:SourceFamily;entries:readonly PlaceEvidence[];kind:SourceKind|null}
export const sourceUses=(keys:readonly string[],from:SourceFamily):SourceUse[]=>keys.map((key)=>({key,from}));
/** Keys are local to an engine family: the id prefix prevents crossed sample records. */
export function sourcesFor(keys:readonly string[],from:SourceFamily='layout',ledger:readonly PlaceEvidence[]=EVIDENCE_LEDGER):PlaceEvidence[] {
  const wanted=new Set(keys);return ledger.filter((entry)=>(from==='bench'?entry.id.startsWith('place-bench-'):entry.id.startsWith('place-layout-')||entry.id.startsWith('place-rule-'))&&entry.sources.some((source)=>source.key!==undefined&&wanted.has(source.key)));
}
export function sourceRecords(uses:readonly SourceUse[],ledger:readonly PlaceEvidence[]=EVIDENCE_LEDGER):SourceRecord[] {
  const seen=new Set<string>();return uses.flatMap(({key,from})=>{
    const entries=sourcesFor([key],from,ledger),kind=Object.entries(PLACE_SOURCES_WITHOUT_ENTRY).find(([id])=>id===key)?.[1]??null;
    const id=kind===null?`${from}:${key}`:key;if(seen.has(id))return[];seen.add(id);
    return entries.length===0&&kind===null?[]:[{key,from,entries,kind}];
  });
}
export function stageSourceUses(proposals:LayoutProposals,load:BenchResult|null):SourceUse[] {
  const layout=proposals.layouts[0];
  const placed=[...(layout?.anchors.flatMap((a)=>[...a.sources,...a.pedicle.sources,...a.lookSources.flatMap((l)=>l.sources)])??[]),...(layout?.rods.flatMap((r)=>r.sources)??[]),...proposals.rules.flatMap((r)=>r.sources),...proposals.selective.flatMap((r)=>r.sources)];
  const loaded=load?.layouts.find((l)=>l.layoutId===layout?.id)?.stops.filter((s)=>s.stop==='rods-in').flatMap((s)=>[...s.estimates.flatMap((e)=>e.sources),...s.parts.flatMap((p)=>p.sources)])??[];
  return [...sourceUses([...new Set(placed)],'layout'),...sourceUses([...new Set(loaded)],'bench')];
}
/** Only the width-table entry supplies the width line, never another table fed by that key. */
export function pedicleOrigin(anchor:Pick<AnchorProposal,'pedicle'|'level'>,age:number|null,ledger:readonly PlaceEvidence[]=EVIDENCE_LEDGER):string|null {
  if(anchor.pedicle.outerWidthMm===null)return null;
  const tables=sourcesFor(anchor.pedicle.sources,'layout',ledger).filter((entry)=>entry.id.startsWith('place-layout-pedicle-widths-')&&entry.id.endsWith('-outer-width'));
  const table=tables[0];if(table!==undefined){const population=populationWords(table,table.sources.find((source)=>source.key!==undefined&&anchor.pedicle.sources.includes(source.key))?.key??'');if(population!=='')return A.origin(population);}
  return ageOrigin(anchor.level,age);
}
const POPULATION_KEYS=[...new Set([...EVIDENCE_LEDGER.flatMap((row)=>row.sources.flatMap((source)=>source.key===undefined?[]:[source.key])),...Object.keys(PLACE_SOURCES_WITHOUT_ENTRY)])];
/** A multi-paper entry has one keyed population segment per paper. Never borrow its neighbour. */
export function populationWords(entry:PlaceEvidence,key:string):string {
  const keys=new Set([...POPULATION_KEYS,...entry.sources.flatMap((source)=>source.key===undefined?[]:[source.key])]);
  const markers=[...keys].flatMap((name)=>{let at=entry.population.indexOf(`${name}:`);while(at>0&&!/\s/.test(entry.population[at-1]??''))at=entry.population.indexOf(`${name}:`,at+1);return at<0?[]:[{name,at,start:at+name.length+1}];}).sort((a,b)=>a.at-b.at);
  const own=markers.findIndex((marker)=>marker.name===key),marker=markers[own];
  if(marker===undefined&&(markers.length>0||entry.sources.length>1))return '';
  const words=marker===undefined?entry.population:entry.population.slice(marker.start,markers[own+1]?.at);
  return (words.split(/\bSample\s*:/)[0]??'').replace(/\s*\[[^\]]*\][\s\S]*$/,'').trim();
}
export function endSourceKeys(proposals:LayoutProposals,end:'upper'|'lower'):string[] {return [...new Set(proposals.rules.filter((rule)=>rule.end===end).flatMap((rule)=>rule.sources))];}
export function ageOrigin(_level:Level,age:number|null):string|null {
  if(age===null)return W.origin;if(age>=12)return A.teenagers;
  return A.published;
}
