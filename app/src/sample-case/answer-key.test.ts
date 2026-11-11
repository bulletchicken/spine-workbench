import type { FilmAnswer, TrueMarks } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { answerKey, rotationVector } from './answer-key';

const key = answerKey();
const film = (kind: FilmAnswer['kind']) => key.films.find((f) => f.kind === kind);
const LEVELS = ['C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];

/** An endplate line's angle on the film, in degrees, from the true marks as published. */
function endplates(marks: TrueMarks): { sup: number; inf: number } {
  const angle = (from: [number, number], to: [number, number]) => (Math.atan2(to[1] - from[1], to[0] - from[0]) * 180) / Math.PI;
  if (marks.kind === 'front') return { sup: angle(marks.supR.at, marks.supL.at), inf: angle(marks.infR.at, marks.infL.at) };
  if (marks.kind === 'side') return { sup: angle(marks.supP.at, marks.supA.at), inf: angle(marks.infP.at, marks.infA.at) };
  return { sup: angle(marks.sup[1].at, marks.sup[0].at), inf: angle(marks.inf[1].at, marks.inf[0].at) };
}

describe('the answer key', () => {
  it('turns rotation matrices into their axis times their angle', () => {
    const t = 0.3;
    expect(rotationVector([[Math.cos(t), -Math.sin(t), 0], [Math.sin(t), Math.cos(t), 0], [0, 0, 1]]).map((v) => Math.round(v * 1e9) / 1e9)).toEqual([0, 0, 0.3]);
    expect(rotationVector([[1, 0, 0], [0, Math.cos(-t), -Math.sin(-t)], [0, Math.sin(-t), Math.cos(-t)]])[0]).toBeCloseTo(-0.3, 9);
    expect(rotationVector([[1, 0, 0], [0, 1, 0], [0, 0, 1]])).toEqual([0, 0, 0]);
  });

  it('poses the 18 bodies of the standard chain from the underside of L5', () => {
    expect(key.poses.map((pose) => pose.level)).toEqual(LEVELS);
    const l5 = key.poses[17];
    // L5's centre is half its 61-pixel height above its underside, at 0.45 mm a pixel.
    expect(l5?.centre[2]).toBeCloseTo(13.725, 9);
    expect(l5?.height).toBeGreaterThanOrEqual(61 * 0.45);
    expect(l5?.height).toBeLessThan(61 * 0.45 * 1.05);
    expect(l5?.topHalfWidth).toBeCloseTo(54 * 0.45, 9);
    expect(key.poses.every((pose, k) => k === 0 || pose.centre[2] < (key.poses[k - 1]?.centre[2] ?? Infinity))).toBe(true);
  });

  it('holds the truth on every film: T11 and T12 apart, T1\'s left pedicle unseen, C7\'s not painted', () => {
    expect(key.films.map((f) => f.kind)).toEqual(['front', 'side', 'bend-left', 'bend-right']);
    for (const f of key.films) expect(f.vertebrae.map((vertebra) => vertebra.level)).toEqual(LEVELS);
    const front = film('front')?.vertebrae ?? [];
    const pedicles = front.flatMap((vertebra) => (vertebra.marks.kind === 'front' ? [[vertebra.level, vertebra.marks.pedL.visible, vertebra.marks.pedR.visible]] : []));
    expect(pedicles.filter(([, left, right]) => left !== true || right !== true)).toEqual([
      ['C7', false, false],
      ['T1', false, true],
    ]);
    expect(film('front')?.bentPoses).toBeNull();
    expect(film('bend-right')?.bentPoses).toHaveLength(18);
    expect(film('bend-left')?.geometry.quarterTurns).toBe(2);
  });

  it('puts c7 at the mean of the true C7 corners on every film', () => {
    for (const f of key.films) {
      const marks = f.vertebrae[0]?.marks;
      if (marks === undefined) throw new Error('every film has C7');
      const corners =
        marks.kind === 'front' ? [marks.supL, marks.supR, marks.infR, marks.infL] : marks.kind === 'side' ? [marks.supA, marks.supP, marks.infP, marks.infA] : [marks.sup[0], marks.sup[1], marks.inf[1], marks.inf[0]];
      const mean = [0, 1].map((axis) => corners.reduce((sum, mark) => sum + (mark.at[axis] ?? 0), 0) / 4);
      expect(f.landmarks.c7.at[0]).toBeCloseTo(mean[0] ?? 0, 6);
      expect(f.landmarks.c7.at[1]).toBeCloseTo(mean[1] ?? 0, 6);
    }
  });

  it('measures the Cobb angles that the published true marks give, brute force', () => {
    const front = film('front')?.vertebrae.map((vertebra) => endplates(vertebra.marks)) ?? [];
    const cobb = (first: number, last: number) => {
      let largest = 0;
      for (let i = first; i <= last; i++) for (let j = i; j <= last; j++) largest = Math.max(largest, Math.abs((front[i]?.sup ?? 0) - (front[j]?.inf ?? 0)));
      return largest;
    };
    expect(key.angles.frontalCobb.map((curve) => curve.region)).toEqual(['proximal-thoracic', 'main-thoracic', 'thoracolumbar-lumbar']);
    expect(key.angles.frontalCobb[1]?.degrees).toBeCloseTo(cobb(5, 12), 1);
    expect(key.angles.frontalCobb[2]?.degrees).toBeCloseTo(cobb(11, 17), 1);
    // The front film visibly curves twice.
    expect(key.angles.frontalCobb[1]?.degrees).toBeGreaterThan(20);
    expect(key.angles.frontalCobb[2]?.degrees).toBeGreaterThan(20);
    const side = film('side')?.vertebrae.map((vertebra) => endplates(vertebra.marks)) ?? [];
    expect(key.angles.kyphosisDegrees).toBeCloseTo(Math.abs((side[4]?.sup ?? 0) - (side[12]?.inf ?? 0)), 1);
  });
});
