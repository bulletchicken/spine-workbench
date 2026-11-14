import { LevelWords } from '@/building-blocks/LevelWords';
import { useStore } from 'zustand';
import type { Band, Measure, MeasureScale } from '@/shapes';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { DAILY_LOADS_WITHHELD, ESTIMATE_NAMES, ESTIMATE_TAGS, PLACE_LOAD_WORDS as W, PLACE_WORDS } from '@/words/place';
import { loadState } from './load-state';
import { placeState } from './place-state';
import { estimatesAt, pastWords, rangeWords, rodsInDrawing, typicalKyphosis, unsettledCount, curveLevel, shownLoadStop, shownStopResult, variationWords } from './load-view';
import { filmSide } from './placement-view';
export function RangeBar({ band, scale }: { band: Band; scale: MeasureScale | undefined }) {
  if (scale === undefined) return null;
  const span = scale.highest - scale.lowest, at = (n: number) => span === 0 ? 50 : Math.max(0,Math.min(100,100*(n-scale.lowest)/span));
  return <svg className="load-range" viewBox="0 0 64 12" aria-hidden="true"><path className="scale" d="M0 6H64"/><path className="band" d={`M${String(at(band.p10)*.64)} 6H${String(at(band.p90)*.64)}`}/><path className="middle" d={`M${String(at(band.p50)*.64)} 2V10`}/></svg>;
}
export function LoadEstimates({ accepted }: { accepted: AcceptedSpine }) {
  const load=useStore(loadState),proposals=useStore(placeState,(s)=>s.result);
  if(load.phase!=='loaded'||load.result===null||proposals===null)return null;
  const result=load.result,at=shownLoadStop(load.stop),layout=proposals.layouts[0],stop=shownStopResult(result,at),estimates=estimatesAt(result,proposals,accepted.fit,at).filter((e)=>!DAILY_LOADS_WITHHELD||e.measure!=='heaviest-rod-swing'),unsettled=unsettledCount(result),source=rodsInDrawing(result,load.batches,accepted.fit).source;
  const name=(part:NonNullable<typeof stop>['parts'][number]['part'])=>part.kind==='anchor'?PLACE_WORDS.pedicleName(part.level,filmSide(part.side,accepted.input.front.marks.namedFor)):part.kind==='rod-span'?W.span(part.from,part.to,filmSide(part.side,accepted.input.front.marks.namedFor)):part.kind==='rod'?PLACE_WORDS.rod(filmSide(part.side,accepted.input.front.marks.namedFor)):ESTIMATE_NAMES[part.which==='above'?'junction-above':'junction-below'];
  return <section className="load-estimates" aria-labelledby="all-estimates-title"><h2 id="all-estimates-title" tabIndex={-1}>{W.allEstimates}</h2>
    {at==='before'?<p>{W.noLoad}</p>:<ul className="load-estimate-list">{estimates.map((e)=><li key={e.measure}><span>{ESTIMATE_NAMES[e.measure]}</span><strong className="num">{rangeWords(e.measure,e.band)}</strong><RangeBar band={e.band} scale={result.scales.find((s)=>s.measure===e.measure)}/><small>{ESTIMATE_TAGS[e.tag]}</small>{variationWords(e.mostlyFrom)!==null&&<p>{variationWords(e.mostlyFrom)}</p>}</li>)}</ul>}
    {unsettled>0&&<p>{W.unsettled(unsettled,result.versions)}</p>}
    {stop!==undefined&&stop.outsideStiffRod>0&&<p>{W.outside(W.denominator(stop.outsideStiffRod,result.versions,unsettled))}</p>}
    {at!=='before'&&stop?.parts.filter((p)=>p.versionsPastStrength>0&&p.part.kind!=='junction').map((p)=><p className="past-strength" key={JSON.stringify(p.part)}><i className="dot" aria-hidden="true"/><LevelWords>{name(p.part)}</LevelWords> {pastWords(p,result)}<small className="load-context">{W.pastContext(p.part.kind==='anchor')}</small><small>{W.partAssumed}</small></p>)}
    {stop?.cyclesPerYear!=null&&<p>{W.stepsYear((Math.floor(stop.cyclesPerYear.p10/100000)/10).toFixed(1),(Math.ceil(stop.cyclesPerYear.p90/100000)/10).toFixed(1))}</p>}
    {layout?.upper==='T1'&&at!=='before'&&!estimates.some((e)=>e.measure==='junction-above')&&<p><LevelWords>{W.noAbove}</LevelWords></p>}
    <div className="panel-note"><p>{W.rangeHolds}</p>{curveLevel(result,proposals)!==null&&<><p>{W.mainCurve}</p>{result.mainCurve===null&&<p>{W.placedCurve}</p>}</>}{source!=='middle'&&<p>{source==='version'?W.fallback:source==='first-version'?W.fallbackFirst:W.noSettled}</p>}{layout!==undefined&&typicalKyphosis(layout)&&<p>{W.kyphosisMissing}</p>}{(accepted.fit.sideFilm===null||proposals.cannotTell.some((c)=>c.code==='twist-not-measured'))&&<p>{W.twistMissing}</p>}<p>{W.allowance}</p>{DAILY_LOADS_WITHHELD?<><p>{W.dailyNotYet}</p><p>{W.trunkNotBalanced}</p></>:<><p>{W.weight}</p><p>{W.steps}</p></>}</div>
  </section>;
}
export function scaleFor(result: NonNullable<ReturnType<typeof loadState.getState>['result']>,measure:Measure): MeasureScale|undefined { return result.scales.find((s)=>s.measure===measure); }
