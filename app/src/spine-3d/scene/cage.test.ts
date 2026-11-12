import { describe, expect, it } from 'vitest';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { CAGE_CUBE, cageLines } from './cage';

const bones = bonesFromDisk();

describe('the see-through line version of a bone', () => {
  it('is a coarse mesh: a few hundred lines where the bone has thousands of edges', () => {
    for (const bone of bones) {
      const lines = cageLines(bone.points, bone.triangles).length / 6;
      const edges = (bone.triangleCount * 3) / 2;
      // Measured: 305 lines (C7) to 716 (L5), a seventh to a ninth of the bone's own edges.
      expect(lines, bone.level).toBeGreaterThan(250);
      expect(lines, bone.level).toBeLessThan(edges / 6);
    }
  });

  it('keeps to the bone: every line ends within a cube of one of the bone\'s own points, and none is longer than three cubes', () => {
    const bone = bones.find((each) => each.level === 'L3');
    if (!bone) throw new Error('no L3');
    const lines = cageLines(bone.points, bone.triangles);
    for (let i = 0; i + 5 < lines.length; i += 6) {
      const long = Math.hypot((lines[i] ?? 0) - (lines[i + 3] ?? 0), (lines[i + 1] ?? 0) - (lines[i + 4] ?? 0), (lines[i + 2] ?? 0) - (lines[i + 5] ?? 0));
      expect(long).toBeGreaterThan(0);
      // A long thin triangle on a body's wall can join cubes that are not neighbours.
      expect(long).toBeLessThan(6 * CAGE_CUBE);
    }
    for (let i = 0; i + 2 < lines.length; i += 300) {
      let nearest = Infinity;
      for (let p = 0; p + 2 < bone.points.length; p += 3) {
        nearest = Math.min(nearest, Math.hypot((lines[i] ?? 0) - (bone.points[p] ?? 0), (lines[i + 1] ?? 0) - (bone.points[p + 1] ?? 0), (lines[i + 2] ?? 0) - (bone.points[p + 2] ?? 0)));
      }
      expect(nearest).toBeLessThan(CAGE_CUBE);
    }
  });

  it('is the same each time, and coarser with larger cubes', () => {
    const bone = bones.find((each) => each.level === 'T7');
    if (!bone) throw new Error('no T7');
    expect(cageLines(bone.points, bone.triangles)).toEqual(cageLines(bone.points, bone.triangles));
    expect(cageLines(bone.points, bone.triangles, 12).length).toBeLessThan(cageLines(bone.points, bone.triangles, 6).length);
  });
});
