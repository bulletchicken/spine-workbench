import { expect, it } from 'vitest';
import { sample, LENGTH } from './timeline';
it('opens the cage before travel, fills after depth and brings the shadow last', () => {
  expect(sample(0).open(0)).toBe(0);
  expect(sample(860 / LENGTH).travel).toBe(0);
  expect(sample(1080 / LENGTH).depth).toBe(0);
  expect(sample(1800 / LENGTH).fill).toBeCloseTo(0);
  expect(sample(2460 / LENGTH).shadow).toBeCloseTo(0);
  const end = sample(1); expect(end.fill).toBe(1); expect(end.depth).toBe(1); expect(end.shadow).toBe(1); expect(end.trips.front).toBe(1);
  for (const p of [.8, .2, 1, 0]) expect(sample(p).trips).toEqual(sample(p).trips);
});
