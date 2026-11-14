import type { AnchorProposal, Band, BenchBatch, BenchResult, BenchStop, Estimate, ImplantLayout, LayoutProposals, Level, Measure, PartResult, SpineFit, StopResult, VertebraPose } from '@/shapes';
import { TEMPLATE_LEVELS } from '@/shapes';
import { placingOf } from '@/spine-3d/placing/place-vertebra';
import { Quaternion, Vector3 } from 'three';
import { PLACE_LOAD_WORDS as W, SHOWN_LOAD_STOPS, DAILY_LOADS_WITHHELD } from '@/words/place';
export function shownLoadStop(stop: BenchStop, shown: readonly BenchStop[] = SHOWN_LOAD_STOPS): BenchStop { return shown.includes(stop) ? stop : 'rods-in'; }
export function stopResult(result: BenchResult | null, stop: BenchStop): StopResult | undefined { return result?.layouts.find((l) => l.layoutId === result.firstId)?.stops.find((s) => s.stop === stop); }
export function unsettledCount(result: BenchResult): number { return result.layouts.find((l) => l.layoutId === result.firstId)?.versionsUnsettled ?? 0; }
export function rangeWords(measure: Measure, band: Band): string {
  const percent = ['heaviest-anchor-demand','heaviest-rod-use','heaviest-rod-swing'].includes(measure), angle = ['main-cobb','kyphosis','apical-twist'].includes(measure), force = measure === 'heaviest-anchor-force';
  const factor = percent ? 100 : 1, places = percent || angle || force ? 0 : 1, grid = 10 ** places;
  const low = (Math.floor(band.p10 * factor * grid) / grid).toFixed(places), high = (Math.ceil(band.p90 * factor * grid) / grid).toFixed(places);
  return W.range(low, high, percent ? '%' : angle ? '°' : force ? ' N' : '×');
}
export function curveLevel(result: BenchResult, proposals: LayoutProposals): Level | null {
  if (result.mainCurve !== null) return result.mainCurve.apex;
  const curve = proposals.curves.reduce<(typeof proposals.curves)[number] | null>((best, c) => best === null || c.versionsMajor > best.versionsMajor ? c : best, null);
  return curve !== null && curve.versionsMajor > 0 ? curve.apex.reduce<(typeof curve.apex)[number]|null>((best,a)=>best===null||a.versions>best.versions?a:best,null)?.level ?? null : null;
}
export function typicalKyphosis(layout: ImplantLayout): boolean { return layout.rods.length === 0 || layout.rods.some((r) => r.targetKyphosisStatus === 'typical'); }
export function estimatesAt(result: BenchResult, proposals: LayoutProposals, fit: SpineFit, stop: BenchStop): Estimate[] {
  if (stop === 'before') return [];
  const layout = proposals.layouts[0]; if (layout === undefined) return [];
  return (stopResult(result, stop)?.estimates ?? []).filter((e) => e.measure === 'main-cobb' ? curveLevel(result,proposals) !== null : e.measure === 'kyphosis' ? !typicalKyphosis(layout) : e.measure === 'apical-twist' ? fit.sideFilm !== null && !proposals.cannotTell.some((c) => c.code === 'twist-not-measured') : true);
}
/** Preserve the fitted bone dimensions; the drawing moves centres and rotations only. */
export function drawingPoses(fit: SpineFit, poses: readonly VertebraPose[] | null): VertebraPose[] {
  return fit.vertebrae.filter((v) => v.level !== 'C7' && TEMPLATE_LEVELS.standard.includes(v.level)).map((v) => { const at = poses?.find((p) => p.level === v.level); return at === undefined ? v.pose : { ...v.pose, centre: at.centre, rotation: at.rotation }; });
}
export function rodsInDrawing(result: BenchResult, batches: readonly BenchBatch[], fit: SpineFit): { poses: VertebraPose[]; source: 'middle' | 'version' | 'fit'; rods: StopResult['rods'] } {
  const rodsIn = stopResult(result, 'rods-in');
  if (rodsIn?.middle != null) return { poses: drawingPoses(fit, rodsIn.middle), source: 'middle', rods: rodsIn.rods };
  const target = rodsIn?.estimates.find((e) => e.measure === 'main-cobb')?.band.p50;
  const runs = batches.flatMap((b) => b.runs).filter((r) => r.layoutId === result.firstId && r.settled).map((r) => ({ version: r.version, stop: r.stops.find((s) => s.stop === 'rods-in') })).filter((r) => r.stop?.poses != null);
  if (target === undefined || runs.length === 0) return { poses: drawingPoses(fit, null), source: 'fit', rods: null };
  runs.sort((a,b) => Math.abs((a.stop?.mainCobbDeg ?? Infinity) - target) - Math.abs((b.stop?.mainCobbDeg ?? Infinity) - target) || a.version - b.version);
  return { poses: drawingPoses(fit, runs[0]?.stop?.poses ?? null), source: 'version', rods: null };
}
export function anchorPoint(anchor: AnchorProposal, fit: SpineFit, poses: readonly VertebraPose[]): [number, number, number] {
  const from = fit.vertebrae.find((v) => v.level === anchor.level)?.pose, to = poses.find((p) => p.level === anchor.level);
  if (from === undefined || to === undefined) return anchor.entryMm;
  return movedPoint(anchor.entryMm,from,to);
}
export function movedPoint(point: readonly [number,number,number], from: VertebraPose, to: VertebraPose): [number,number,number] {
  const r = new Vector3(...from.rotation), angle = r.length(), inverse = angle === 0 ? new Quaternion() : new Quaternion().setFromAxisAngle(r.normalize(), angle).invert();
  const own = new Vector3(...point).sub(new Vector3(...from.centre)).applyQuaternion(inverse);
  return [...placingOf(to).toPatient(own.toArray())];
}

export function anchorLoad(parts: readonly PartResult[], anchor: AnchorProposal): PartResult | undefined { return parts.find((p) => p.part.kind === 'anchor' && p.part.level === anchor.level && p.part.side === anchor.side); }
export interface LoadLabel { id: string; level: Level; measure: Measure; band: Band; tag: Estimate['tag']; anchor?: number }
export function loadLabels(result: BenchResult, proposals: LayoutProposals, fit: SpineFit, stop: BenchStop, anchorStop: BenchStop = 'bending-forward'): LoadLabel[] {
  if (stop === 'before') return [];
  const estimates = estimatesAt(result,proposals,fit,stop), layout = proposals.layouts[0], apex = curveLevel(result,proposals), labels: LoadLabel[] = [];
  if (layout === undefined) return labels;
  const curve = estimates.find((e) => e.measure === 'main-cobb'), kyphosis = estimates.find((e) => e.measure === 'kyphosis');
  if (curve !== undefined && apex !== null) labels.push({id:'main-cobb',level:apex,measure:'main-cobb',band:curve.band,tag:curve.tag});
  if (kyphosis !== undefined && fit.vertebrae.some((v) => v.level === 'T5')) labels.push({id:'kyphosis',level:'T5',measure:'kyphosis',band:kyphosis.band,tag:kyphosis.tag});
  const reference = stopResult(result,anchorStop)?.parts ?? [];
  let chosen = -1, demand = -Infinity;
  layout.anchors.forEach((a,i) => { const d=anchorLoad(reference,a)?.demand; if(a.kind !== 'none' && d != null && d.p50 > demand) { chosen=i; demand=d.p50; } });
  const anchor=layout.anchors[chosen], own=anchor===undefined?undefined:anchorLoad(stopResult(result,stop)?.parts??[],anchor);
  if(anchor!==undefined && own?.demand!=null) labels.push({id:'anchor',level:anchor.level,measure:'heaviest-anchor-demand',band:own.demand,tag:'assumed',anchor:chosen});
  return labels;
}
export function pastWords(part: PartResult, result: BenchResult): string | null {
  if (part.versionsPastStrength <= 0 || part.part.kind === 'junction') return null;
  return W.past(part.part.kind,W.denominator(part.versionsPastStrength,result.versions,unsettledCount(result)));
}

export function shownStopResult(result: BenchResult | null, stop: BenchStop): StopResult | undefined {
  const at=stopResult(result,shownLoadStop(stop));
  return at===undefined||!DAILY_LOADS_WITHHELD?at:{...at,parts:at.parts.filter((p)=>p.part.kind!=='rod'),estimates:at.estimates.filter((e)=>e.measure!=='heaviest-rod-swing'),cyclesPerYear:null};
}
