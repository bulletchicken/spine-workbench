import { describe, expect, it } from 'vitest';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { TEMPLATE_LEVELS, type FilmKind, type FilmMarks, type Level } from '@/shapes';
import { lightShareAt, replayEnds, revealPlan, shownAt } from './find-replay';

const times = { lead: 700, travel: 4400, hold: 560 };
const light = { height: 200, edge: 60, filmHeight: 1300 };
function marks(kind: FilmKind, levels: readonly Level[], height = 1300): FilmMarks {
  const base = smallStory().pass.marks.find((film) => film.film === kind);
  const v = base?.vertebrae[0];
  if (base === undefined || v === undefined) throw new Error('missing outline');
  return { ...base, vertebrae: levels.map((level, id) => {
    const top = (100 + id * 60) * height / 1300;
    const point = (x: number, y: number) => ({ at: [x, y] as [number, number], source: 'auto' as const, pinned: false, inferred: false });
    return { ...v, id, level, marks: { kind: 'bending' as const, lines: { sup: [point(10, top), point(30, top)] as [ReturnType<typeof point>, ReturnType<typeof point>], inf: [point(10, top + 20), point(30, top + 20)] as [ReturnType<typeof point>, ReturnType<typeof point>] } } };
  }) };
}

describe('replaying the returned outlines', () => {
  it('plans seventeen outlines at their true shares and counts only Front', () => {
    const levels = TEMPLATE_LEVELS.standard.filter((level) => level !== 'C7');
    const plan = revealPlan([marks('front', levels), marks('side', levels)], { front: { width: 650, height: 1300 }, side: { width: 650, height: 1300 } }, light);
    if (plan === null) throw new Error('missing plan');
    expect(plan.counted).toHaveLength(17);
    levels.forEach((_, i) => {
      expect(plan.at.get(`front:${String(i)}`)).toBeCloseTo((60 + 60 * i) / 1080);
      expect(plan.at.get(`side:${String(i)}`)).toBe(plan.at.get(`front:${String(i)}`));
    });
    expect(shownAt(plan, times, 0).count).toBe(0);
    expect(shownAt(plan, times, 699).shown.size).toBe(0);
    expect(shownAt(plan, times, 2900).count).toBe(9);
    const last = 700 + 4400 * plan.last;
    expect(shownAt(plan, times, last + 0.001)).toMatchObject({ count: 17, allShown: true });
    expect(replayEnds(plan, times)).toBe(last + 560);
  });
  it('puts a single outline halfway through the light, and scales the light with its film', () => {
    const plan = revealPlan([marks('front', ['T1'], 3900)], { front: { width: 1000, height: 3900 } }, light);
    expect(plan?.at.get('front:0')).toBe(0.5);
    expect(plan?.light.front).toEqual({ from: 130, to: 490, height: 600 });
  });
  it('uses a partner outline’s own share when Front has no name for it, leaving C7 out', () => {
    const plan = revealPlan([marks('front', ['T1']), marks('side', ['C7', 'T1', 'T2'])], {}, light);
    expect(plan?.at.has('side:0')).toBe(false);
    expect(plan?.at.get('side:1')).toBe(0.5);
    expect(plan?.at.get('side:2')).toBeCloseTo(2 / 3);
    expect(plan?.last).toBeCloseTo(2 / 3);
    expect(revealPlan([marks('front', ['C7'])], {}, light)).toBeNull();
    expect(revealPlan([marks('side', ['T1'])], {}, light)).toBeNull();
  });
  it('keeps the light off outside its one journey', () => {
    expect([0, 699, 700, 2900, 5100, 5101].map((time) => lightShareAt(times, time))).toEqual([null, null, 0, 0.5, 1, null]);
  });
});
