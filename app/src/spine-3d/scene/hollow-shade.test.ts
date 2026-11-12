import { describe, expect, it } from 'vitest';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { hollowShade, pointNormals } from './hollow-shade';

const bones = bonesFromDisk();

describe('the shade a bone carries in its hollows', () => {
  it('is a share for every point, the same each time it is worked out', () => {
    const bone = bones.find((each) => each.level === 'T7');
    if (!bone) throw new Error('no T7');
    const open = hollowShade(bone.points, bone.triangles);
    expect(open.length).toBe(bone.pointCount);
    for (const share of open) {
      expect(share).toBeGreaterThanOrEqual(0);
      expect(share).toBeLessThanOrEqual(1);
    }
    expect(hollowShade(bone.points, bone.triangles)).toEqual(open);
  });

  it('leaves the front wall of the body open and darkens the walls of the canal', () => {
    for (const level of ['T3', 'T7', 'T12', 'L3'] as const) {
      const bone = bones.find((each) => each.level === level);
      if (!bone) throw new Error(`no ${level}`);
      const open = hollowShade(bone.points, bone.triangles);
      const normals = pointNormals(bone.points, bone.triangles);
      const front: number[] = [];
      const canal: number[] = [];
      for (let p = 0; p < bone.pointCount; p += 1) {
        const [x, y, z] = [bone.points[p * 3] ?? 0, bone.points[p * 3 + 1] ?? 0, bone.points[p * 3 + 2] ?? 0];
        // The front wall of the body: forward of its middle and facing forward.
        if (x > 0.5 * bone.halfDepth && (normals[p * 3] ?? 0) > 0.7 && Math.abs(z) < 0.5 * bone.height) front.push(open[p] ?? 0);
        // The back of the body faces into the canal: just behind the body, near the midline, facing back.
        if (x < -0.75 * bone.halfDepth && x > -1.4 * bone.halfDepth && Math.abs(y) < 0.35 * bone.halfWidth && (normals[p * 3] ?? 0) < -0.5) {
          canal.push(open[p] ?? 0);
        }
      }
      const mean = (values: number[]) => values.reduce((sum, value) => sum + value, 0) / values.length;
      expect(front.length, level).toBeGreaterThan(5);
      expect(canal.length, level).toBeGreaterThan(2);
      // Measured: the front wall is fully open (1.00) on all four; the canal's front wall 0.61 to 0.79.
      expect(mean(front), level).toBeGreaterThan(0.95);
      expect(mean(canal), level).toBeLessThan(0.85);
    }
  });
});
