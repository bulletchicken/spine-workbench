// A straight-spine fixture for exercising the measurement door in dev.
// This stand-in does not calculate geometry or sample versions. The real
// engine is the single source of measurements and bands.

import type { Level, NotMeasured, PairReading, Reading, ReadingsInput, SpineReadings } from '@/shapes';

export function sampleReadings(input: ReadingsInput): SpineReadings {
  const measured = (): Reading => ({ kind: 'measured', valueDeg: 0, band: { p10: 0, p50: 0, p90: 0 } });
  const absent = (reason: NotMeasured, typicalDeg: number | null = null): Reading => ({ kind: 'not-measured', reason, typicalDeg });
  const levels = input.fit.vertebrae.map((v) => v.level);
  const hasSide = input.fit.sideFilm !== null;
  const sideMissing: NotMeasured = hasSide ? 'not-on-side-film' : 'no-side-film';
  const sideMeasured = (level: Level): boolean => hasSide && input.fit.vertebrae.some((v) => v.level === level && !v.inferred.depth);
  const sagittal = (upper: Level, lower: Level): Reading => {
    const fitted = levels.includes(upper) && levels.includes(lower);
    if (!hasSide) return absent('no-side-film', fitted ? 0 : null);
    if (!fitted) return absent('not-fitted');
    return sideMeasured(upper) && sideMeasured(lower) ? measured() : absent(sideMissing, 0);
  };
  const pairs = (side: boolean): PairReading[] => levels.flatMap((upper, index) => levels.slice(index + 1).map((lower) => ({
    upper, lower, angle: side && (!sideMeasured(upper) || !sideMeasured(lower)) ? absent(sideMissing, 0) : measured(),
  })));
  return {
    tier: hasSide ? 'two-films' : 'front-film-only',
    versions: input.versions,
    vertebrae: input.fit.vertebrae.map((v) => ({
      level: v.level,
      coronalTilt: measured(),
      forwardLean: sideMeasured(v.level) ? measured() : absent(sideMissing, 0),
      twist: v.inferred.twist ? absent('twist-not-measured', 0) : measured(),
    })),
    sagittal: {
      kyphosisT2T5: sagittal('T2', 'T5'),
      kyphosisT5T12: sagittal('T5', 'T12'),
      kyphosisT10L2: sagittal('T10', 'L2'),
      lordosisL1L5: sagittal('L1', 'L5'),
      lordosisL1S1: !hasSide ? absent('no-side-film') : !levels.includes('L1') ? absent('not-fitted') : !sideMeasured('L1') ? absent(sideMissing) : absent('no-sacrum'),
    },
    curves: [],
    between: { levels, front: pairs(false), side: pairs(true) },
  };
}
