import type { FilmPlacement, FittedVertebra, ReadingsInput, SpineFit } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { startStandInEngine } from './stand-in-engine';

function input(side: boolean): ReadingsInput {
  const placement: FilmPlacement = { mmPerPx: [0.5, 0.5], originPx: [100, 200] };
  const vertebra = (level: FittedVertebra['level']): FittedVertebra => ({
    level,
    pose: { level, centre: [0, 0, 10], rotation: [0, 0, 0], height: 20, wedgeFront: 0, wedgeSide: 0, topHalfWidth: 15, topHalfDepth: 10, bottomHalfWidth: 15, bottomHalfDepth: 10 },
    angles: { sideTiltRad: 0, forwardLeanRad: 0, twistRad: 0 },
    sd: { centreMm: [0, 0, 0], anglesRad: [0, 0, 0] },
    inferred: { depth: !side, width: false, twist: true },
    rmsMm: { front: 0, side: side ? 0 : null },
    cornerTiltRad: 0,
  });
  const fit: SpineFit = {
    vertebrae: [vertebra('T2'), vertebra('T5')], sideScale: 1, sideOffsetMm: 0, originLevel: 'T5', iterations: 1,
    converged: true, stopReason: 'small-step', rmsMm: 0, lengths: 'measured', typicalCurveDeg: null,
    frontFilm: placement, sideFilm: side ? placement : null,
  };
  return { fit, front: null, side: null, bendLeft: null, bendRight: null, versions: 32, seed: 1 };
}

describe('the stand-in measurement fixture', () => {
  it('answers through the client without an open film and keeps absent views explicit', async () => {
    const engine = startStandInEngine();
    try {
      const result = await engine.measureSpine(input(false));
      expect(result.tier).toBe('front-film-only');
      expect(result.between.levels).toEqual(['T2', 'T5']);
      expect(result.between.front).toHaveLength(1);
      expect(result.between.side[0]?.angle).toEqual({ kind: 'not-measured', reason: 'no-side-film', typicalDeg: 0 });
      expect(result.vertebrae[0]?.twist).toEqual({ kind: 'not-measured', reason: 'twist-not-measured', typicalDeg: 0 });
      expect(result.sagittal.kyphosisT2T5).toEqual({ kind: 'not-measured', reason: 'no-side-film', typicalDeg: 0 });
      expect(result.sagittal.kyphosisT5T12).toEqual({ kind: 'not-measured', reason: 'no-side-film', typicalDeg: null });
    } finally {
      engine.stop();
    }
  });

  it('keeps missing side geometry unread even when a side film is present', async () => {
    const engine = startStandInEngine();
    try {
      const request = input(true);
      for (const v of request.fit.vertebrae) if (v.level === 'T2') v.inferred.depth = true;
      const complete = await engine.measureSpine(input(true));
      const result = await engine.measureSpine(request);
      expect(result.tier).toBe('two-films');
      expect(result.between.front).toEqual(complete.between.front);
      expect(result.vertebrae.map((v) => v.coronalTilt)).toEqual(complete.vertebrae.map((v) => v.coronalTilt));
      expect(result.vertebrae[1]?.forwardLean).toEqual({ kind: 'measured', valueDeg: 0, band: { p10: 0, p50: 0, p90: 0 } });
      expect(result.vertebrae[0]?.forwardLean).toEqual({ kind: 'not-measured', reason: 'not-on-side-film', typicalDeg: 0 });
      expect(result.between.side[0]?.angle).toEqual({ kind: 'not-measured', reason: 'not-on-side-film', typicalDeg: 0 });
      expect(result.sagittal.kyphosisT2T5).toEqual({ kind: 'not-measured', reason: 'not-on-side-film', typicalDeg: 0 });
      const unseenL1 = request.fit.vertebrae[0];
      if (!unseenL1) throw new Error('the fixture has a vertebra');
      unseenL1.level = 'L1';
      unseenL1.pose.level = 'L1';
      request.fit.vertebrae = [unseenL1];
      expect((await engine.measureSpine(request)).sagittal.lordosisL1S1).toEqual({ kind: 'not-measured', reason: 'not-on-side-film', typicalDeg: null });
    } finally {
      engine.stop();
    }
  });

  it('answers the same straight fixture repeatably for two films', async () => {
    const engine = startStandInEngine();
    try {
      const first = await engine.measureSpine(input(true));
      expect(first).toEqual(await engine.measureSpine(input(true)));
      expect(first.sagittal.kyphosisT2T5).toEqual({ kind: 'measured', valueDeg: 0, band: { p10: 0, p50: 0, p90: 0 } });
      expect(first.sagittal.lordosisL1S1).toEqual({ kind: 'not-measured', reason: 'not-fitted', typicalDeg: null });
    } finally {
      engine.stop();
    }
  });
});
