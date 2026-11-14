import { LevelWords } from '@/building-blocks/LevelWords';
import { PLACE_SOURCE_WORDS as W } from '@/words/place';
import { handState } from './hand-state';
import { placeState } from './place-state';
import { populationWords,sourceRecords,type SourceRecord,type SourceUse } from './placement-sources';
export function openSources():void {
  handState.setState({why:null});placeState.setState({sheet:true});
  requestAnimationFrame(()=>{const heading=document.getElementById('place-sources-title');heading?.scrollIntoView({block:'start'});heading?.focus({preventScroll:true});});
}
function shortCitation(citation:string):string {
  const words=/^([^ ,]+).*?\b((?:19|20)\d{2})\b/.exec(citation);return words===null?citation:`${words[1]??''} ${words[2]??''}`;
}
export function SourceLinks({uses}:{uses:readonly SourceUse[]}) {
  const records=sourceRecords(uses);if(records.length===0)return null;
  const citations=[...new Set(records.flatMap((record)=>record.entries.flatMap((entry)=>entry.sources.filter((s)=>s.key===record.key).map((s)=>s.citation))))];
  const kinds=[...new Set(records.flatMap((record)=>record.entries.length===0&&record.kind!==null?[record.kind]:[]))];
  return <span className="place-source-links">{citations.map((citation)=><button className="link" type="button" key={citation} onClick={openSources}>{shortCitation(citation)}</button>)}{kinds.map((kind)=><span key={kind}>{W.kinds[kind]}</span>)}</span>;
}
export function PlaceSources({records}:{records:readonly SourceRecord[]}) {
  if(records.length===0)return null;
  const ordered=[...records].sort((a,b)=>Number(a.kind!==null)-Number(b.kind!==null));
  return <section className="place-sources"><h2 id="place-sources-title" tabIndex={-1}>{W.title}</h2><ul className="place-sheet-list">{ordered.map((record)=>{
    const sources=record.entries.flatMap((entry)=>entry.sources.filter((s)=>s.key===record.key)),citations=[...new Set(sources.map((s)=>s.citation))];
    return <li key={`${record.from}:${record.key}`} data-source-family={record.from} data-source-kind={record.kind??undefined}>
      {citations.length===0&&record.kind!==null?<p>{W.kinds[record.kind]}</p>:<>
        {citations.map((citation)=><p key={citation}><LevelWords>{citation}</LevelWords></p>)}
        <details><summary>{W.tables(record.entries.length)}</summary><ul>{record.entries.map((entry)=><li key={entry.id}><p><LevelWords>{entry.feature}</LevelWords></p><p><LevelWords>{populationWords(entry,record.key)}</LevelWords></p>{entry.sampleSize!==undefined&&entry.sources[0]?.key===record.key&&<p>{W.sample(entry.sampleSize)}</p>}{entry.sources.filter((s)=>s.key===record.key&&s.reading!==undefined).map((s)=><p key={s.reading}>{W[s.reading==='full-text'?'full':s.reading==='abstract'?'abstract':'secondary']}</p>)}{!entry.checked&&<p>{W.unchecked}</p>}</li>)}</ul></details>
        {[...new Set(record.entries.map((entry)=>populationWords(entry,record.key)).filter(Boolean))].map((population)=><p key={population}><LevelWords>{population}</LevelWords></p>)}
        {[...new Set(record.entries.flatMap((entry)=>entry.sampleSize===undefined||entry.sources[0]?.key!==record.key?[]:[entry.sampleSize]))].map((size)=><p key={size}>{W.sample(size)}</p>)}
        {[...new Set(sources.flatMap((s)=>s.reading===undefined?[]:[s.reading]))].map((reading)=><p key={reading}>{W[reading==='full-text'?'full':reading==='abstract'?'abstract':'secondary']}</p>)}
        {record.entries.some((entry)=>!entry.checked)&&<p>{W.unchecked}</p>}
      </>}
    </li>;
  })}</ul></section>;
}
