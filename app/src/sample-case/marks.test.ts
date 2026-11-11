import { describe, expect, it } from 'vitest';
import { boneOf } from './anatomy';
import { passMarks, withIdsFrom } from './marks';

describe('the marks the pass leaves', () => {
  it('numbers 18 bodies on each film from 0, film after film', () => {
    const marks = passMarks();
    expect(marks.map((film) => film.film)).toEqual(['front', 'side', 'bend-left', 'bend-right']);
    expect(marks.map((film) => film.vertebrae.length)).toEqual([18, 18, 18, 18]);
    expect(marks.flatMap((film) => film.vertebrae.map((vertebra) => vertebra.id))).toEqual(Array.from({ length: 72 }, (_, k) => k));
    const raised = withIdsFrom(marks, 5);
    expect(raised[1]?.vertebrae[0]?.id).toBe(23);
  });

  it('puts every corner where the mock draws it, half a pixel off, with the left on the image right', () => {
    const t6 = passMarks()[0]?.vertebrae[6];
    const bone = boneOf('front', 6);
    expect(t6?.marks.kind).toBe('front');
    if (t6?.marks.kind !== 'front') return;
    expect(t6.marks.points.supL.at[0]).toBeCloseTo(bone.q[1].x - 0.5, 2);
    expect(t6.marks.points.supR.at[0]).toBeCloseTo(bone.q[0].x - 0.5, 2);
    expect(t6.marks.points.infL.at[1]).toBeCloseTo(bone.q[2].y - 0.5, 2);
    expect(t6.marks.points.supL.at[0]).toBeGreaterThan(t6.marks.points.supR.at[0]);
  });

  it('leaves T3 short of its bone on the patient\'s left of the front film only', () => {
    const [front, side] = passMarks();
    const t3 = boneOf('front', 3);
    const corner = front?.vertebrae[3]?.marks;
    if (corner?.kind !== 'front') throw new Error('T3 has front marks');
    // 17 pixels in along the endplate at the top.
    expect(Math.hypot(corner.points.supL.at[0] - (t3.q[1].x - 0.5), corner.points.supL.at[1] - (t3.q[1].y - 0.5))).toBeCloseTo(17, 1);
    const sideT3 = side?.vertebrae[3]?.marks;
    if (sideT3?.kind !== 'side') throw new Error('T3 has side marks');
    expect(sideT3.points.supA.at[0]).toBeCloseTo(boneOf('side', 3).q[1].x - 0.5, 2);
  });

  it('stores bending left upside down: its C7 near the bottom of the stored film', () => {
    const bendLeft = passMarks()[2];
    expect(bendLeft?.landmarks.c7?.at[1]).toBeGreaterThan(1400);
    expect(bendLeft?.vertebrae[0]?.level).toBe('C7');
    const sup = bendLeft?.vertebrae[0]?.marks;
    if (sup?.kind !== 'bending') throw new Error('bending films have endplate lines');
    // Upright, the upper endplate is above the lower; stored, it is below it.
    expect(sup.lines.sup[0].at[1]).toBeGreaterThan(sup.lines.inf[0].at[1]);
  });
});
