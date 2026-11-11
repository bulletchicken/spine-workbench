import type { Stage, WatchFrame } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { keyframesOnly } from './keyframes-only';
import { passMarks, quadOf } from './marks';

/** The contract's rule, from docs/contracts/watch-frames.md: what a step loop may thin. */
const mayBeThinned = (frame: WatchFrame) =>
  frame.kind === 'joint-progress' || (frame.kind === 'evidence' && !frame.final && frame.map !== 'wavefront');

const STAGE_ORDER: readonly Stage[] = ['evidence', 'centre-line', 'strip', 'chain', 'naming', 'confidence'];

describe('the keyframes-only pass', () => {
  const marks = passMarks();
  const frames = keyframesOnly(marks);

  it('holds no frame the watch-frame rule lets a step loop thin', () => {
    expect(frames.filter(mayBeThinned)).toEqual([]);
  });

  it('starts all six stages on every film, in order, and ends each with final outlines', () => {
    for (const film of ['front', 'side', 'bend-left', 'bend-right'] as const) {
      const own = frames.filter((frame) => frame.film === film);
      const started = own.filter((frame) => frame.kind === 'stage-started').map((frame) => frame.stage);
      expect(started).toEqual(STAGE_ORDER);
      expect(own.map((frame) => STAGE_ORDER.indexOf(frame.stage))).toEqual([...own.map((frame) => STAGE_ORDER.indexOf(frame.stage))].sort((a, b) => a - b));
      expect(own.at(-1)).toMatchObject({ kind: 'outlines', final: true });
    }
  });

  it('locks the chain from the sacrum upward and names it top to bottom', () => {
    const front = marks[0];
    const locked = frames.find((frame) => frame.kind === 'chain-locked' && frame.film === 'front');
    const named = frames.find((frame) => frame.kind === 'levels-named' && frame.film === 'front');
    if (locked?.kind !== 'chain-locked' || named?.kind !== 'levels-named') throw new Error('both frames are there');
    expect(locked.bodies.map((body) => body.level)).toEqual([...named.levels].reverse());
    expect(named.levels).toEqual(front?.vertebrae.map((vertebra) => vertebra.level));
    // Upright, the first body to lock is the lowest on the film.
    expect(locked.bodies[0]?.sup.a[1]).toBeGreaterThan(locked.bodies[1]?.sup.a[1] ?? Infinity);
    expect(named.naming.l5ConfirmedOn).toBeNull();
  });

  it('ends with outlines equal to the marks', () => {
    marks.forEach((film) => {
      const outlines = frames.find((frame) => frame.kind === 'outlines' && frame.film === film.film);
      if (outlines?.kind !== 'outlines') throw new Error('every film has outlines');
      expect(outlines.quads).toEqual(film.vertebrae.map((vertebra) => ({ level: vertebra.level, corners: quadOf(vertebra.marks).map((mark) => mark.at) })));
    });
  });
});
