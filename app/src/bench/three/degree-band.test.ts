import { expect, it } from 'vitest';
import { degreeBand } from './degree-band';
it('rounds the nominal away from zero and the larger raw distance upwards', () => {
  for (const [valueDeg, band, value, radius] of [
    [40, [36, 42, 45], '40°', '± 5°'], [-20, [-25.1, -19, -17], '−20°', '± 6°'],
    [-0.5, [-1, -0.5, 0], '−1°', '± 1°'], [20, [22, 23, 25], '20°', '± 5°'],
  ] as const) expect(degreeBand({ kind: 'measured', valueDeg, band: { p10: band[0], p50: band[1], p90: band[2] } })).toEqual({ value, band: radius });
  expect(degreeBand({ kind: 'measured', valueDeg: -0.1, band: { p10: -1, p50: 0, p90: 1 } }).value).toBe('0°');
});
