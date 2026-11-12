import { describe, expect, it } from 'vitest';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { bodyFrameOf, intoBody } from './bone-body';
import { RIM_POINTS, boneLandmarks, landmarksOf } from './body-landmarks';
import type { BodyNumbers } from './endplates';
import { archJoinOf, shapeBone, shaperFor } from './shape-bone';
import { type Vec3, length, sub } from './vectors';

const bones = bonesFromDisk();

/** Bodies unlike any bone's own: taller, wider, shallower, wedged both ways, unequal endplates. */
const FITTED: BodyNumbers[] = [
  { height: 20, wedgeFront: 0, wedgeSide: 0, topHalfWidth: 15, topHalfDepth: 10, bottomHalfWidth: 15, bottomHalfDepth: 10 },
  { height: 26, wedgeFront: 0.12, wedgeSide: -0.2, topHalfWidth: 21, topHalfDepth: 15, bottomHalfWidth: 23, bottomHalfDepth: 17 },
  { height: 14.5, wedgeFront: -0.08, wedgeSide: 0.15, topHalfWidth: 12, topHalfDepth: 8.5, bottomHalfWidth: 13.5, bottomHalfDepth: 9.5 },
  { height: 19, wedgeFront: 0.25, wedgeSide: 0.3, topHalfWidth: 24, topHalfDepth: 9, bottomHalfWidth: 18, bottomHalfDepth: 16 },
];

const pointsOf = (points: Float32Array): Vec3[] => {
  const all: Vec3[] = [];
  for (let i = 0; i + 2 < points.length; i += 3) all.push([points[i] ?? 0, points[i + 1] ?? 0, points[i + 2] ?? 0]);
  return all;
};

/** Six times the volume a closed surface encloses: positive when it faces outwards. */
function sixVolumes(points: Vec3[], triangles: Uint16Array): number {
  let sum = 0;
  for (let i = 0; i + 2 < triangles.length; i += 3) {
    const [a, b, c] = [points[triangles[i] ?? 0], points[triangles[i + 1] ?? 0], points[triangles[i + 2] ?? 0]];
    if (!a || !b || !c) continue;
    sum += a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0]);
  }
  return sum;
}

describe('a bone shaped to a fitted vertebra', () => {
  it('has the fitted endplates: the same centres, the four half-sizes and both wedges', () => {
    let worst = 0;
    for (const bone of bones) {
      const own = boneLandmarks(bone.body);
      for (const fitted of FITTED) {
        const shape = shaperFor(bone.body, fitted);
        const goal = landmarksOf(fitted);
        for (const plate of ['top', 'bottom'] as const) {
          for (const point of RIM_POINTS) {
            worst = Math.max(worst, length(sub(shape(own[plate][point]), goal[plate][point])));
          }
        }
      }
    }
    // The landmarks are carried exactly; what is left is rounding.
    expect(worst).toBeLessThan(1e-9);
  });

  it('is left as it was when the fitted vertebra is the bone\'s own body', () => {
    for (const bone of bones) {
      const frame = bodyFrameOf(bone.body);
      const shaped = pointsOf(shapeBone(bone, frame.numbers));
      const before = pointsOf(bone.points).map((point) => intoBody(frame, point));
      let worst = 0;
      shaped.forEach((point, i) => {
        worst = Math.max(worst, length(sub(point, before[i] ?? [0, 0, 0])));
      });
      // Float32 holds a point 100 mm out to about 1e-5 mm.
      expect(worst, bone.level).toBeLessThan(1e-4);
    }
  });

  it('keeps the arch in proportion: behind the body it grows with the width alone', () => {
    for (const bone of bones) {
      const frame = bodyFrameOf(bone.body);
      const own = frame.numbers;
      // Half as wide again, twice as deep, a third taller: only the width may reach the arch.
      const fitted: BodyNumbers = {
        ...own,
        height: own.height * 1.33,
        topHalfWidth: own.topHalfWidth * 1.5,
        bottomHalfWidth: own.bottomHalfWidth * 1.5,
        topHalfDepth: own.topHalfDepth * 2,
        bottomHalfDepth: own.bottomHalfDepth * 2,
      };
      const shape = shaperFor(bone.body, fitted);
      const join = archJoinOf(bone);
      const behind = pointsOf(bone.points).filter((point) => intoBody(frame, point)[0] < join.wall - join.length);
      expect(behind.length, bone.level).toBeGreaterThan(50);
      const first = behind[0] ?? [0, 0, 0];
      const firstShaped = shape(first);
      for (const point of behind) {
        const before = length(sub(point, first));
        if (before < 1) continue;
        expect(length(sub(shape(point), firstShaped)) / before, bone.level).toBeCloseTo(1.5, 9);
      }
    }
  });

  it('joins the arch where the pedicles are: above the middle of the body, below its top plate', () => {
    for (const bone of bones) {
      const { height } = archJoinOf(bone);
      const own = bodyFrameOf(bone.body).numbers;
      expect(height, bone.level).toBeGreaterThan(0);
      expect(height, bone.level).toBeLessThan(0.5 * own.height);
    }
  });

  it('never tears or folds: every triangle keeps its side, and no edge is stretched far past what was asked', () => {
    const ratios: number[] = [];
    for (const bone of bones) {
      const own = bodyFrameOf(bone.body).numbers;
      // Bodies a fit could ask of this bone: a quarter taller or shorter, wider or narrower, deeper or
      // shallower, each on its own, with wedges of its own.
      const asks: { fitted: BodyNumbers; stretches: number[] }[] = [
        [1.25, 0.85, 0.85, 1.2, 1.2, 0.15, -0.2],
        [0.8, 1.25, 1.1, 0.85, 0.9, -0.1, 0.2],
        [1.3, 1.3, 1.3, 1.3, 1.3, 0, 0],
        [1, 1.2, 1.2, 0.8, 0.8, 0.2, 0.2],
      ].map(([up = 1, topAcross = 1, bottomAcross = 1, topDeep = 1, bottomDeep = 1, wedgeFront = 0, wedgeSide = 0]) => ({
        fitted: {
          height: own.height * up,
          wedgeFront,
          wedgeSide,
          topHalfWidth: own.topHalfWidth * topAcross,
          bottomHalfWidth: own.bottomHalfWidth * bottomAcross,
          topHalfDepth: own.topHalfDepth * topDeep,
          bottomHalfDepth: own.bottomHalfDepth * bottomDeep,
        },
        stretches: [up, topAcross, bottomAcross, topDeep, bottomDeep],
      }));
      for (const { fitted, stretches } of asks) {
        const before = pointsOf(bone.points);
        const after = pointsOf(shapeBone(bone, fitted));
        expect(sixVolumes(after, bone.triangles), bone.level).toBeGreaterThan(0);
        let most = 0;
        let least = Infinity;
        for (let i = 0; i + 2 < bone.triangles.length; i += 3) {
          for (const [from, to] of [[0, 1], [1, 2], [2, 0]] as const) {
            const [a, b] = [bone.triangles[i + from] ?? 0, bone.triangles[i + to] ?? 0];
            const was = length(sub(before[a] ?? [0, 0, 0], before[b] ?? [0, 0, 0]));
            const is = length(sub(after[a] ?? [0, 0, 0], after[b] ?? [0, 0, 0]));
            most = Math.max(most, is / was);
            least = Math.min(least, is / was);
          }
        }
        ratios.push(most / Math.max(...stretches), Math.min(...stretches) / least);
      }
    }
    // Measured: where a body made a quarter taller and a sixth narrower meets its arch, an edge is
    // at most 1.75 times shorter than the smallest stretch asked for. Everywhere else it is closer.
    expect(Math.max(...ratios)).toBeLessThan(1.9);
  });

  it('follows its landmarks: the shaped body reaches the fitted half-sizes at its rims', () => {
    const fitted: BodyNumbers = { height: 22, wedgeFront: 0, wedgeSide: 0, topHalfWidth: 18, topHalfDepth: 14, bottomHalfWidth: 18, bottomHalfDepth: 14 };
    for (const bone of bones) {
      const shaped = pointsOf(shapeBone(bone, fitted));
      for (const plate of [1, -1]) {
        // The body's points near one plate, from its middle forward: behind that the pedicles and rib joints begin.
        const near = shaped.filter((p) => plate * p[2] > 0.35 * fitted.height && plate * p[2] < 0.5 * fitted.height + 0.5 && p[0] > -3);
        const left = Math.max(...near.map((p) => p[1]));
        const right = Math.min(...near.map((p) => p[1]));
        const front = Math.max(...near.map((p) => p[0]));
        // Measured: the front wall within 0.46 mm of the oval's end on every bone. A real rim is not an
        // oval, and the upper thoracic bodies narrow toward the front: their sides are within 3.1 mm,
        // those of T7 to L5 within 0.7 mm.
        expect(Math.abs(front - 14), bone.level).toBeLessThan(0.6);
        expect(Math.abs(left - 18), bone.level).toBeLessThan(3.5);
        expect(Math.abs(right + 18), bone.level).toBeLessThan(3.5);
      }
    }
  });
});
