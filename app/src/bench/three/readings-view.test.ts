import { expect, it } from 'vitest';
import type { FilmToFit, Reading, SpineReadings } from '@/shapes';
import { readingsView } from './readings-view';
const measured: Reading = { kind: 'measured', valueDeg: -20, band: { p10: -25.1, p50: -19, p90: -17 } };
const missing: Reading = { kind: 'not-measured', reason: 'no-side-film', get typicalDeg(): number { throw new Error('must never read'); } };
function answer(): SpineReadings { return { tier: 'front-film-only', versions: 32, vertebrae: [],
 sagittal: { kyphosisT2T5: missing, kyphosisT5T12: measured, kyphosisT10L2: missing, lordosisL1L5: measured, lordosisL1S1: { kind: 'not-measured', reason: 'no-sacrum', typicalDeg: 999 } },
 curves: [{ region: 'main-thoracic', convex: 'left', upperEnd: 'T5', lowerEnd: 'T12', apex: 'T8', upperEndSeen: [], lowerEndSeen: [], apexSeen: [], versionsFound: 32, cobb: measured, apexTwist: { kind: 'not-measured', reason: 'twist-not-measured', typicalDeg: 999 }, bendingCobb: missing }],
 between: { levels: ['T5', 'T8', 'T12', 'L1', 'L5'], front: [], side: [] } }; }
it('keeps front sizes, signed side curves, fixed ends and unique omissions without reading typical values', () => {
 const a = answer(); const curve = a.curves[0]; if (curve === undefined) throw new Error('no curve'); curve.apexTwist = missing;
 const front = { geometry: { imageRight: 'patient-left' } } as FilmToFit;
 const view = readingsView(a, front);
 expect(view.rows.map((r) => r.value)).toEqual(['20°', '−20°', '−20°']);
 expect(view.rows[0]?.about).toBe('Cobb angle T5 to T12, bulging toward the right of the picture');
 expect(view.rows[0]?.levels).toEqual(['T5', 'T8', 'T12']);
 expect(view.reasons).toEqual(['The lower curve is read to L5; the sacrum is not marked.', 'Side curves need a side film.']);
 expect(JSON.stringify(view)).not.toContain('999');
 front.geometry.imageRight = 'patient-right'; expect(readingsView(a, front).rows[0]?.about).toContain('toward the left');
});
