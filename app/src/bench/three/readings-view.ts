import { TEMPLATE_LEVELS, type Level, type Reading, type SpineReadings, type FilmToFit } from '@/shapes';
import { READINGS_WORDS as W, REASON_WORDS, REGION_NAMES, curveAbout, pictureDirection, twistAbout } from '@/words/spine-readings';
import { degreeBand } from './degree-band';
export interface MeasurementRow { key: string; name: string; value: string; band: string; about: string; levels: readonly Level[] }
export function readingsView(result: SpineReadings, front: FilmToFit) {
  if (result.versions !== 32) throw new Error('reading-versions');
  const known = (level: Level) => level === 'C7' || TEMPLATE_LEVELS.standard.includes(level);
  type SafeReading = Extract<Reading, { kind: 'measured' }> | { kind: 'not-measured'; reason: import('@/shapes').NotMeasured };
  const clean = (r: Reading): SafeReading => {
    if (r.kind === 'not-measured') {
      if (r.reason === 'not-fitted') throw new Error('reading-not-fitted');
      return { kind: r.kind, reason: r.reason };
    }
    const { valueDeg, band } = r;
    if (![valueDeg, band.p10, band.p50, band.p90].every(Number.isFinite) || band.p10 > band.p50 || band.p50 > band.p90) throw new Error('reading-band');
    return { kind: r.kind, valueDeg, band: { ...band } };
  };
  const rows: MeasurementRow[] = [];
  const reasons = new Set<string>();
  const reason = (r: SafeReading) => { if (r.kind === 'not-measured') { const words = REASON_WORDS[r.reason]; if (words !== null) reasons.add(words); } };
  const add = (key: string, name: string, r: Reading, about: string, levels: readonly Level[], magnitude = false) => {
    const safe = clean(r);
    if (!levels.every(known)) throw new Error('reading-level');
    if (safe.kind === 'measured') rows.push({ key, name, ...degreeBand(safe, magnitude), about, levels });
    else reason(safe);
  };
  const inclusive = (upper: Level, lower: Level) => result.between.levels.filter((level) => TEMPLATE_LEVELS.standard.indexOf(level) >= TEMPLATE_LEVELS.standard.indexOf(upper) && TEMPLATE_LEVELS.standard.indexOf(level) <= TEMPLATE_LEVELS.standard.indexOf(lower));
  const direction = (bodyLeft: boolean) => pictureDirection(bodyLeft === (front.geometry.imageRight === 'patient-right'));
  for (const curve of result.curves) add(JSON.stringify([curve.region, curve.convex, curve.upperEnd, curve.lowerEnd, curve.apex]), REGION_NAMES[curve.region], curve.cobb, curveAbout(curve.upperEnd, curve.lowerEnd, direction(curve.convex === 'left')), inclusive(curve.upperEnd, curve.lowerEnd), true);
  if (result.curves.length === 0) reasons.add(REASON_WORDS['no-curve'] ?? '');
  const s = result.sagittal;
  add('kyphosis', W.kyphosis, s.kyphosisT5T12, W.kyphosisAbout, inclusive('T5', 'T12'));
  const sacrum = s.lordosisL1S1.kind === 'measured';
  reason(clean(s.lordosisL1S1));
  add('lordosis', W.lordosis, sacrum ? s.lordosisL1S1 : s.lordosisL1L5, W.lordosisAbout(sacrum), inclusive('L1', 'L5'));
  const largest = result.curves[0];
  if (largest !== undefined) add('twist', W.twist, largest.apexTwist, twistAbout(largest.apex, largest.apexTwist.kind === 'measured' && largest.apexTwist.valueDeg !== 0 ? direction(largest.apexTwist.valueDeg > 0) : ''), [largest.apex], true);
  else for (const v of result.vertebrae) if (v.twist.kind === 'not-measured' && v.twist.reason === 'twist-not-measured') reason(v.twist);
  const vertebrae = result.vertebrae.map((v) => { if (!known(v.level)) throw new Error('reading-level'); return { level: v.level, coronalTilt: clean(v.coronalTilt), forwardLean: clean(v.forwardLean), twist: clean(v.twist) }; });
  const pairs = { front: result.between.front.map((p) => ({ ...p, angle: clean(p.angle) })), side: result.between.side.map((p) => ({ ...p, angle: clean(p.angle) })) };
  return { rows, reasons: [...reasons], byLevel: (level: Level) => vertebrae.find((v) => v.level === level), between: (plane: 'front' | 'side', upper: Level, lower: Level) => pairs[plane].find((p) => p.upper === upper && p.lower === lower) };
}
