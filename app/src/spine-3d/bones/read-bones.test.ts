import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { type Bone, type BoneManifest, readBones } from './read-bones';

const FOLDER = fileURLToPath(new URL('../../../public/bones/', import.meta.url));
const manifest = JSON.parse(readFileSync(`${FOLDER}vertebrae.json`, 'utf8')) as BoneManifest;
const file = readFileSync(`${FOLDER}vertebrae.bin`);
const bytes = file.buffer.slice(file.byteOffset, file.byteOffset + file.byteLength);
const bones = readBones(manifest, bytes);

/** The points of a bone that lie within its body's footprint, seen from above. */
function overTheBody(bone: Bone): number[][] {
  const points: number[][] = [];
  for (let i = 0; i < bone.points.length; i += 3) {
    const [x, y, z] = [bone.points[i] ?? 0, bone.points[i + 1] ?? 0, bone.points[i + 2] ?? 0];
    if (Math.abs(x) < 0.9 * bone.halfDepth && Math.abs(y) < 0.9 * bone.halfWidth) points.push([x, y, z]);
  }
  return points;
}

describe('the shipped vertebra shapes', () => {
  it('are the eighteen vertebrae from C7 to L5, in order', () => {
    expect(bones.map((bone) => bone.level)).toEqual([
      'C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5',
    ]);
  });

  it('carry the credit line the licence asks for', () => {
    expect(manifest.credit).toBe(
      'BodyParts3D, © The Database Center for Life Science licensed under CC Attribution 4.0 International',
    );
    expect(readFileSync(fileURLToPath(new URL('../../../../NOTICE', import.meta.url)), 'utf8').replace(/\s+/g, ' ')).toContain(
      manifest.credit,
    );
  });

  it('fill the file exactly, with no bone overlapping the next', () => {
    let at = 0;
    for (const bone of bones) {
      expect(bone.pointsAt).toBe(at);
      at += bone.pointCount * 12;
    }
    for (const bone of bones) {
      expect(bone.trianglesAt).toBe(at);
      at += bone.triangleCount * 6;
    }
    expect(at).toBe(manifest.byteLength);
  });

  it('are closed surfaces: every edge is shared by exactly two triangles', () => {
    for (const bone of bones) {
      const uses = new Map<number, number>();
      for (let i = 0; i < bone.triangles.length; i += 3) {
        const corners = [bone.triangles[i] ?? 0, bone.triangles[i + 1] ?? 0, bone.triangles[i + 2] ?? 0];
        corners.forEach((a, k) => {
          const b = corners[(k + 1) % 3] ?? 0;
          expect(a).toBeLessThan(bone.pointCount);
          const edge = Math.min(a, b) * 65536 + Math.max(a, b);
          uses.set(edge, (uses.get(edge) ?? 0) + 1);
        });
      }
      expect([...uses.values()].every((count) => count === 2), bone.level).toBe(true);
    }
  });

  it('face outwards: each surface encloses a positive volume', () => {
    for (const bone of bones) {
      let sixVolumes = 0;
      const at = (n: number): [number, number, number] => [bone.points[n * 3] ?? 0, bone.points[n * 3 + 1] ?? 0, bone.points[n * 3 + 2] ?? 0];
      for (let i = 0; i < bone.triangles.length; i += 3) {
        const [a, b, c] = [at(bone.triangles[i] ?? 0), at(bone.triangles[i + 1] ?? 0), at(bone.triangles[i + 2] ?? 0)];
        sixVolumes += a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0]);
      }
      // A vertebra is between 5 and 80 cubic centimetres.
      expect(sixVolumes / 6000, bone.level).toBeGreaterThan(5);
      expect(sixVolumes / 6000, bone.level).toBeLessThan(80);
    }
  });

  it('sit in their own frame: the body is centred, its front at halfDepth, its plates at half the height', () => {
    for (const bone of bones) {
      const body = overTheBody(bone);
      const top = Math.max(...body.map((p) => p[2] ?? 0));
      const bottom = Math.min(...body.map((p) => p[2] ?? 0));
      // Measured: the rims of the plates lie within 1.6 mm of half the height on every bone.
      expect(Math.abs(top - bone.height / 2), bone.level).toBeLessThan(2);
      expect(Math.abs(bottom + bone.height / 2), bone.level).toBeLessThan(2);
      let front = -Infinity;
      for (let i = 0; i < bone.points.length; i += 3) front = Math.max(front, bone.points[i] ?? 0);
      expect(Math.abs(front - bone.halfDepth), bone.level).toBeLessThan(0.5);
    }
  });

  it('grow down the spine as vertebrae do', () => {
    const height = (level: string): number => bones.find((bone) => bone.level === level)?.height ?? 0;
    expect(height('T1')).toBeLessThan(height('T6'));
    expect(height('T6')).toBeLessThan(height('T12'));
    expect(height('T12')).toBeLessThan(height('L3'));
    for (const bone of bones) {
      expect(bone.height, bone.level).toBeGreaterThan(10);
      expect(bone.height, bone.level).toBeLessThan(35);
      expect(bone.halfWidth * 2, bone.level).toBeGreaterThan(bone.height);
    }
  });

  it('refuses bytes that are not the file the list describes', () => {
    expect(() => readBones(manifest, bytes.slice(0, bytes.byteLength - 2))).toThrow(/bytes/);
  });
});
