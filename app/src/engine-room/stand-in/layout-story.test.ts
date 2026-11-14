import type { RebuildInput, SpineFit } from '@/shapes';
import front from '@/spine-3d/made-up-case/front-only-spine.json';
import { describe, expect, it } from 'vitest';
import { startStandInEngine } from './stand-in-engine';

const input: RebuildInput = {
  layout: {
    fit: front.fit as unknown as SpineFit,
    front: null,
    bendLeft: null,
    bendRight: null,
    child: { ageYears: null, weightKg: null, activity: null },
    naming: 'as-read',
    versions: 32,
    seed: 1,
    pedicleChildShare: null,
  },
  upper: 'T8',
  lower: 'T9',
  anchors: [],
  rodMaterial: 'cobalt-chrome',
  rodDiameterMm: 5.5,
};

describe('the stand-in placement fixture', () => {
  it('answers a rebuild through the client with a fresh placement marked changed by hand', async () => {
    const engine = startStandInEngine();
    try {
      const result = await engine.rebuildLayout(input);
      expect(result.id).toBe('changed-by-hand');
      expect(result.change).toEqual({ kind: 'changed-by-hand' });
      expect(result.anchors.map(({ level, side }) => [level, side])).toEqual([['T8', 'left'], ['T8', 'right'], ['T9', 'left'], ['T9', 'right']]);
      expect(result.rods.map(({ side }) => side)).toEqual(['left', 'right']);
      expect(result.rods.every(({ standingPointsMm }) => standingPointsMm.length === 0)).toBe(true);
      expect(result.anchors.every(({ screwHere, widerThanPedicle, lookSources }) => screwHere === null && widerThanPedicle.length === 0 && lookSources.length === 0)).toBe(true);
      const again = await engine.rebuildLayout(input);
      expect(again).toEqual(result);
      result.anchors.length = 0;
      expect((await engine.rebuildLayout(input)).anchors).toHaveLength(4);
    } finally {
      engine.stop();
    }
  });
});
