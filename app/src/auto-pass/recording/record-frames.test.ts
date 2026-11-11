import type { AutoPassResult, WatchFrame } from '@/shapes';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { filmRecording, filmStrip, readyBeats, recordFrame, recordFrames, recordResult, startRecording } from './record-frames';
import { TEST_STRIP, testBodies, testFrames } from './test-frames';

const PASS = ['front', 'side'] as const;

function both(): WatchFrame[] {
  return [...testFrames('front'), ...testFrames('side')];
}

function only<K extends WatchFrame['kind']>(frames: WatchFrame[], kind: K): Extract<WatchFrame, { kind: K }>[] {
  return frames.filter((frame): frame is Extract<WatchFrame, { kind: K }> => frame.kind === kind);
}

/** The two films' frames merged, each film's in its own order, with `picks` choosing which goes next. */
function interleave(front: WatchFrame[], side: WatchFrame[], picks: boolean[]): WatchFrame[] {
  const merged: WatchFrame[] = [];
  let f = 0;
  let s = 0;
  for (const pickFront of picks) {
    const next = (pickFront && f < front.length) || s >= side.length ? front[f++] : side[s++];
    if (next !== undefined) merged.push(next);
  }
  merged.push(...front.slice(f), ...side.slice(s));
  return merged;
}

/** Every frame but the ones `drop` picks out. */
function without(frames: WatchFrame[], drop: (frame: WatchFrame) => boolean): WatchFrame[] {
  return frames.filter((frame) => !drop(frame));
}

describe('recording the pass', () => {
  it('gives the same recording for any interleaving of the two films', () => {
    const inOrder = recordFrames(PASS, both());
    const count = both().length;
    fc.assert(
      fc.property(fc.array(fc.boolean(), { minLength: count, maxLength: count }), (picks) => {
        expect(recordFrames(PASS, interleave(testFrames('front'), testFrames('side'), picks))).toEqual(inOrder);
      }),
      { numRuns: 40 },
    );
  });

  it('keeps the finished edge, ridge and ring maps, and lets every other map go by', () => {
    const recording = recordFrames(PASS, both());
    const front = filmRecording(recording, 'front');
    const side = filmRecording(recording, 'side');
    expect(Object.keys(front?.maps ?? {}).sort()).toEqual(['phase-edge', 'ridge', 'ring']);
    expect(Object.keys(side?.maps ?? {}).sort()).toEqual(['phase-edge', 'ridge']);

    // The bitmap is the finished tile's, not the preview's that came before it.
    const finished = only(testFrames('front'), 'evidence').find((frame) => frame.map === 'ridge' && frame.final);
    const grey = Array.from(front?.maps.ridge?.rgba ?? []).filter((_, i) => i % 4 === 0);
    expect(grey).toEqual(Array.from(finished?.tile.data ?? [-1]));
  });

  it('keeps every wavefront tile in the order of its step', () => {
    const front = filmRecording(recordFrames(PASS, both()), 'front');
    expect(front?.wavefront.map((tile) => tile.step)).toEqual([2, 3, 4, 5]);
  });

  it('keeps each value as the frame carried it', () => {
    const frames = testFrames('front');
    const front = filmRecording(recordFrames(PASS, frames), 'front');
    const lines = only(frames, 'centre-line');
    expect(front?.coarseLine).toEqual(lines.find((frame) => !frame.refined)?.points);
    expect(front?.refinedLine).toEqual(lines.find((frame) => frame.refined)?.points);
    expect(front?.width).toEqual(only(frames, 'width')[0]?.samples);
    const zones = only(frames, 'end-zones')[0];
    expect(front?.endZones).toEqual({ top: zones?.top, bottom: zones?.bottom, topCropped: zones?.topCropped, bottomCropped: zones?.bottomCropped });
    expect(front?.sweeps).toEqual(only(frames, 'sweep').map(({ level, heat, tentative }) => ({ level, heat, tentative })));
    expect(front?.locked).toEqual(only(frames, 'chain-locked')[0]?.bodies);
    const named = only(frames, 'levels-named')[0];
    expect(front?.named).toEqual({ naming: named?.naming, levels: named?.levels });
    expect(front?.confidence).toEqual(only(frames, 'confidence')[0]?.levels);
    expect(front?.outlines).toEqual(only(frames, 'outlines')[0]?.quads);
    expect(filmRecording(recordFrames(PASS, testFrames('side')), 'side')?.warnings).toEqual(only(testFrames('side'), 'warning').map((frame) => frame.warning));
  });

  it('joins the strip and reads the profile as little-endian floats', () => {
    const front = filmRecording(recordFrames(PASS, both()), 'front');
    const strip = front === null ? null : filmStrip(front);
    expect(strip?.image.rows).toBe(TEST_STRIP.rows);
    expect(strip?.image.columns).toBe(TEST_STRIP.columns);
    expect(front?.stripBlocks).toEqual([]);
    // Row 0 lies above the first body (a gap: −0.5); row 10 is 24 px down,
    // inside C7 (bone: 0.8). Both are exact in f32 only as their nearest
    // f32, so compare with Math.fround.
    expect(strip?.barcode.values[0]).toBe(Math.fround(-0.5));
    expect(strip?.barcode.values[10]).toBe(Math.fround(0.8));
  });

  it('keeps only the final outlines', () => {
    const early: WatchFrame = { kind: 'outlines', film: 'front', stage: 'confidence', quads: [], final: false };
    const recording = recordFrame(startRecording(PASS), early);
    expect(filmRecording(recording, 'front')?.outlines).toBeNull();
  });

  it('takes nothing from frames of a film outside the pass', () => {
    const empty = startRecording(['front']);
    expect(recordFrames(['front'], testFrames('side'))).toEqual(empty);
  });

  it('holds the answer once the pass returns', () => {
    const result: AutoPassResult = { caseVersion: 4, marks: [], twoFilms: null, warnings: [{ code: 'tight-curve', film: 'front', level: 'T8', region: null }] };
    expect(recordResult(recordFrames(PASS, both()), result).result).toBe(result);
    expect(startRecording(PASS).result).toBeNull();
  });
});

describe('which beats are ready', () => {
  it('is all six once every frame of both films is in', () => {
    expect(readyBeats(recordFrames(PASS, both()))).toBe(6);
  });

  it('is none before any frame', () => {
    expect(readyBeats(startRecording(PASS))).toBe(0);
  });

  // Each row withholds one frame a beat needs, from one film only, and
  // says how many beats can still play: the ones before it.
  const cases: [string, (frame: WatchFrame) => boolean, number][] = [
    ['the side film’s finished edge map', (f) => f.film === 'side' && f.kind === 'evidence' && f.map === 'phase-edge' && f.final, 0],
    ['the front film’s finished ridge map', (f) => f.film === 'front' && f.kind === 'evidence' && f.map === 'ridge' && f.final, 0],
    ['the start of the side film’s centre line stage', (f) => f.film === 'side' && f.kind === 'stage-started' && f.stage === 'centre-line', 0],
    ['the front film’s end zones', (f) => f.film === 'front' && f.kind === 'end-zones', 1],
    ['the front film’s refined centre line', (f) => f.film === 'front' && f.kind === 'centre-line' && f.refined, 1],
    ['the side film’s coarse centre line', (f) => f.film === 'side' && f.kind === 'centre-line' && !f.refined, 1],
    ['one block of the front strip', (f) => f.film === 'front' && f.kind === 'strip-rows' && f.firstRow === 128, 2],
    ['the side film’s strip shape', (f) => f.film === 'side' && f.kind === 'strip-shape', 2],
    ['the front film’s barcode', (f) => f.film === 'front' && f.kind === 'barcode', 2],
    ['the side film’s locked chain', (f) => f.film === 'side' && f.kind === 'chain-locked', 3],
    ['the front film’s names', (f) => f.film === 'front' && f.kind === 'levels-named', 4],
    ['the side film’s confidence', (f) => f.film === 'side' && f.kind === 'confidence', 5],
    ['the front film’s final outlines', (f) => f.film === 'front' && f.kind === 'outlines', 5],
  ];
  it.each(cases)('stops before the beat that needs %s', (_name, drop, ready) => {
    expect(readyBeats(recordFrames(PASS, without(both(), drop)))).toBe(ready);
  });

  it('never passes a beat whose data is missing, even when later beats have theirs', () => {
    const noShape = without(both(), (f) => f.film === 'front' && f.kind === 'strip-shape');
    const recording = recordFrames(PASS, noShape);
    expect(filmRecording(recording, 'front')?.locked).toEqual(testBodies('front').slice().reverse());
    expect(readyBeats(recording)).toBe(2);
  });

  it('does not wait for a ring map on a film that sends none', () => {
    expect(readyBeats(recordFrames(['side'], testFrames('side')))).toBe(6);
  });

  it('refuses a barcode whose rows do not match the strip', () => {
    const short = both().map((frame) => (frame.kind === 'barcode' && frame.film === 'front' ? { ...frame, values: frame.values.slice(4), highPass: frame.highPass.slice(4) } : frame));
    expect(readyBeats(recordFrames(PASS, short))).toBe(2);
  });
});
