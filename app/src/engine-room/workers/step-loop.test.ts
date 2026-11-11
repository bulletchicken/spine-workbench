import type { Level, PreviewTile, Px, Stage, Step, WatchFrame } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { mayBeThinned, nextWorkUnits, runSteps, type StepClock, type Stepper } from './step-loop';

const AT: Px = [10, 20];
const TILE: PreviewTile = { width: 1, height: 1, origin: AT, stepAcross: [1, 0], stepDown: [0, 1], encoding: 'unit', data: new Uint8Array([128]) };
const BODY = { level: 'T1' as Level, sup: { a: AT, b: AT }, inf: { a: AT, b: AT }, inferred: false };
const NAMING = {
  template: 'standard' as const,
  chosenBy: 'engine' as const,
  alternatives: [],
  fromTop: null,
  fromBottom: null,
  flags: [],
  l5ConfirmedOn: null,
};

function evidence(map: 'ridge' | 'phase-edge' | 'ring' | 'wavefront', final: boolean, step = 1): WatchFrame {
  return { kind: 'evidence', film: 'front', stage: 'evidence', map, step, steps: 9, final, scaleMm: 2, tile: TILE };
}

function joint(round: number): WatchFrame {
  return { kind: 'joint-progress', film: 'front', stage: 'chain', start: 0, round, upper: 2, lower: 1 };
}

const started: WatchFrame = { kind: 'stage-started', film: 'front', stage: 'evidence' };

// Every frame kind, with whether it may be thinned: the rule written out
// by hand, as engine/shapes/tests/which_frames_may_be_thinned.rs has it.
// A kind added to WatchFrame without a row here fails to compile.
const DECISIONS: { [K in WatchFrame['kind']]: [string, WatchFrame, boolean][] } = {
  'stage-started': [['stage-started', started, false]],
  evidence: [
    ['unfinished ridge map', evidence('ridge', false), true],
    ['unfinished phase edges', evidence('phase-edge', false), true],
    ['unfinished ring map', evidence('ring', false), true],
    ['finished ridge map', evidence('ridge', true), false],
    ['spreading search', evidence('wavefront', false), false],
    ['finished search', evidence('wavefront', true), false],
  ],
  'end-zones': [
    [
      'end zones',
      { kind: 'end-zones', film: 'front', stage: 'centre-line', top: { left: 0, top: 0, right: 8, bottom: 8 }, bottom: { left: 0, top: 0, right: 8, bottom: 8 }, topCropped: false, bottomCropped: false },
      false,
    ],
  ],
  'centre-line': [
    ['coarse centre line', { kind: 'centre-line', film: 'front', stage: 'centre-line', points: [AT], refined: false }, false],
    ['refined centre line', { kind: 'centre-line', film: 'front', stage: 'centre-line', points: [AT], refined: true }, false],
  ],
  width: [['width', { kind: 'width', film: 'front', stage: 'centre-line', samples: [{ left: AT, right: AT, confidence: 1 }] }, false]],
  'strip-shape': [['strip shape', { kind: 'strip-shape', film: 'front', stage: 'strip', centres: [AT], normals: [[1, 0]], columnPx: 1, columns: 1 }, false]],
  'strip-rows': [
    [
      'strip rows',
      { kind: 'strip-rows', film: 'front', stage: 'strip', firstRow: 0, rows: 1, columns: 1, image: new Uint8Array(1), valid: new Uint8Array(1) },
      false,
    ],
  ],
  barcode: [['barcode', { kind: 'barcode', film: 'front', stage: 'strip', values: new Uint8Array(4), highPass: new Uint8Array(4) }, false]],
  sweep: [['sweep', { kind: 'sweep', film: 'front', stage: 'chain', level: 'T1', heat: new Uint8Array(256), tentative: [BODY] }, false]],
  'template-scored': [['template scored', { kind: 'template-scored', film: 'front', stage: 'chain', template: 'standard', score: 1, chance: 0.5 }, false]],
  'joint-progress': [['joint progress', joint(1), true]],
  'chain-locked': [['chain locked', { kind: 'chain-locked', film: 'front', stage: 'chain', bodies: [BODY] }, false]],
  'levels-named': [['levels named', { kind: 'levels-named', film: 'front', stage: 'naming', naming: NAMING, levels: ['T1'] }, false]],
  confidence: [['confidence', { kind: 'confidence', film: 'front', stage: 'confidence', levels: [{ level: 'T1', score: 0.9, band: 'high' }] }, false]],
  outlines: [
    ['outlines on the way', { kind: 'outlines', film: 'front', stage: 'confidence', quads: [{ level: 'T1', corners: [AT, AT, AT, AT] }], final: false }, false],
    ['final outlines', { kind: 'outlines', film: 'front', stage: 'confidence', quads: [{ level: 'T1', corners: [AT, AT, AT, AT] }], final: true }, false],
  ],
  warning: [['warning', { kind: 'warning', film: 'front', stage: 'evidence', warning: { code: 'low-evidence', film: 'front', level: null, region: null } }, false]],
};

describe('which frames may be thinned', () => {
  it.each(Object.values(DECISIONS).flat())('%s', (_label, frame, thinned) => {
    expect(mayBeThinned(frame)).toBe(thinned);
  });
});

describe('work units', () => {
  it.each([
    [1, 0.5, 2, 'a quick slice at most doubles'],
    [16, 8, 16, 'a slice on target keeps its units'],
    [100, 16, 50, 'a slow slice scales down'],
    [100, 100, 25, 'a very slow slice falls to a quarter, no further'],
    [10, 0, 20, 'a slice too quick to time doubles'],
    [1, 1000, 1, 'never below one unit'],
  ])('%d units in %d ms → %d (%s)', (units, elapsed, next) => {
    expect(nextWorkUnits(units, elapsed)).toBe(next);
  });
});

/** A clock that only moves when told, and counts its pauses. */
function fakeClock(): StepClock & { at: number; pauses: number } {
  const clock = {
    at: 0,
    pauses: 0,
    now: () => clock.at,
    pause: () => {
      clock.pauses += 1;
      return Promise.resolve();
    },
  };
  return clock;
}

interface Slice {
  ms: number;
  frames?: WatchFrame[];
  stage?: Stage;
  fraction?: number;
}

/** A pass that plays slices in order, each taking `ms` on the clock. */
function scriptedPass(clock: { at: number }, slices: Slice[], answer = 'answer') {
  const units: number[] = [];
  const pass = {
    freed: 0,
    finished: 0,
    units,
    advance(workUnits: number): Step {
      units.push(workUnits);
      const slice = slices[units.length - 1];
      if (slice === undefined) throw new Error('advanced past the end');
      clock.at += slice.ms;
      return { done: units.length === slices.length, stage: slice.stage ?? 'evidence', fraction: slice.fraction ?? 0, frames: slice.frames ?? [] };
    },
    finish() {
      pass.finished += 1;
      return answer;
    },
    free() {
      pass.freed += 1;
    },
  };
  return pass;
}

function recorder() {
  const sent: { frames: WatchFrame[]; progress: [Stage, number, number][] } = { frames: [], progress: [] };
  return sent;
}

describe('the step loop', () => {
  it('scales the work units toward an 8 ms slice', async () => {
    const clock = fakeClock();
    // Each unit costs half a millisecond, so 16 units make a slice.
    const units: number[] = [];
    let slices = 0;
    const pass: Stepper<string> = {
      advance(workUnits) {
        units.push(workUnits);
        clock.at += workUnits * 0.5;
        slices += 1;
        return { done: slices === 8, stage: 'evidence', fraction: slices / 8, frames: [] };
      },
      finish: () => 'done',
      free: () => undefined,
    };
    await runSteps(pass, { progress: () => undefined, frame: () => undefined }, () => false, clock);
    expect(units).toEqual([1, 2, 4, 8, 16, 16, 16, 16]);
    expect(clock.pauses).toBe(7);
  });

  it('thins frames to thirty a second, newest kept, never out of order', async () => {
    const clock = fakeClock();
    const sent = recorder();
    const key: WatchFrame = { kind: 'stage-started', film: 'front', stage: 'centre-line' };
    const e = [1, 2, 3, 4].map((step) => evidence('ridge', false, step));
    const pass = scriptedPass(clock, [
      { ms: 10, frames: [e[0] as WatchFrame] }, // t=10: the first goes at once
      { ms: 10, frames: [e[1] as WatchFrame] }, // t=20: held
      { ms: 10, frames: [e[2] as WatchFrame] }, // t=30: replaces the held one
      { ms: 10, frames: [e[3] as WatchFrame] }, // t=40: replaces it again
      { ms: 10, frames: [key] }, // t=50: 40 ms since the last, so the held one goes first
      { ms: 10 },
    ]);
    const outcome = await runSteps(pass, { progress: () => undefined, frame: (frame) => sent.frames.push(frame) }, () => false, clock);
    expect(outcome).toEqual({ kind: 'done', output: 'answer' });
    expect(sent.frames).toEqual([e[0], e[3], key]);
  });

  it('drops a held frame whose turn has not come when a keyframe arrives', async () => {
    const clock = fakeClock();
    const sent = recorder();
    const e1 = evidence('ridge', false, 1);
    const e2 = evidence('ridge', false, 2);
    const pass = scriptedPass(clock, [{ ms: 10, frames: [e1] }, { ms: 10, frames: [e2, started] }, { ms: 10 }]);
    await runSteps(pass, { progress: () => undefined, frame: (frame) => sent.frames.push(frame) }, () => false, clock);
    expect(sent.frames).toEqual([e1, started]);
  });

  it('sends a held frame once its turn comes, even with no new frame', async () => {
    const clock = fakeClock();
    const sent = recorder();
    const j1 = joint(1);
    const j2 = joint(2);
    const pass = scriptedPass(clock, [{ ms: 10, frames: [j1] }, { ms: 10, frames: [j2] }, { ms: 10 }, { ms: 20 }, { ms: 10 }]);
    const times: number[] = [];
    await runSteps(
      pass,
      {
        progress: () => undefined,
        frame: (frame) => {
          sent.frames.push(frame);
          times.push(clock.at);
        },
      },
      () => false,
      clock,
    );
    expect(sent.frames).toEqual([j1, j2]);
    expect(times).toEqual([10, 50]);
  });

  it('holds a thinnable frame 30 ms after the last, and sends it once 33⅓ ms have passed', async () => {
    const clock = fakeClock();
    const sent = recorder();
    const e1 = evidence('ridge', false, 1);
    const e2 = evidence('ridge', false, 2);
    // t=10: sent. t=40: 30 ms on, under a thirtieth of a second, so held. t=43: 33 ms on, still held. t=44: 34 ms on, sent.
    const pass = scriptedPass(clock, [{ ms: 10, frames: [e1] }, { ms: 30, frames: [e2] }, { ms: 3 }, { ms: 1 }, { ms: 10 }]);
    const times: number[] = [];
    await runSteps(
      pass,
      {
        progress: () => undefined,
        frame: (frame) => {
          sent.frames.push(frame);
          times.push(clock.at);
        },
      },
      () => false,
      clock,
    );
    expect(sent.frames).toEqual([e1, e2]);
    expect(times).toEqual([10, 44]);
  });

  it('drops a held frame at the end if its turn has not come', async () => {
    const clock = fakeClock();
    const sent = recorder();
    const e1 = evidence('ring', false, 1);
    const e2 = evidence('ring', false, 2);
    const pass = scriptedPass(clock, [{ ms: 10, frames: [e1] }, { ms: 10, frames: [e2] }]);
    await runSteps(pass, { progress: () => undefined, frame: (frame) => sent.frames.push(frame) }, () => false, clock);
    expect(sent.frames).toEqual([e1]);
  });

  it('keeps every keyframe however fast they come', async () => {
    const clock = fakeClock();
    const sent = recorder();
    const finished = evidence('ridge', true);
    const wave = evidence('wavefront', false);
    const frames = [started, finished, wave, started, wave];
    const pass = scriptedPass(clock, [{ ms: 1, frames }, { ms: 1 }]);
    await runSteps(pass, { progress: () => undefined, frame: (frame) => sent.frames.push(frame) }, () => false, clock);
    expect(sent.frames).toEqual(frames);
  });

  it('sends progress at most ten times a second', async () => {
    const clock = fakeClock();
    const sent = recorder();
    const slices: Slice[] = Array.from({ length: 9 }, (_, i) => ({ ms: 30, stage: 'strip', fraction: (i + 1) / 10 }));
    const pass = scriptedPass(clock, slices);
    await runSteps(pass, { progress: (stage, fraction) => sent.progress.push([stage, fraction, clock.at]), frame: () => undefined }, () => false, clock);
    // Slices end at 30, 60, … 270. Progress goes after the first, then
    // after the first slice to end 100 ms or more later (150); the ninth
    // slice ends the pass, and its answer says the rest.
    expect(sent.progress).toEqual([
      ['strip', 0.1, 30],
      ['strip', 0.5, 150],
    ]);
  });

  it('stops between slices when cancelled, frees the pass and gives no answer', async () => {
    const clock = fakeClock();
    let cancel = false;
    const pass = scriptedPass(clock, [{ ms: 5 }, { ms: 5 }, { ms: 5 }, { ms: 5 }]);
    const pausing: StepClock = {
      now: () => clock.at,
      pause: () => {
        if (pass.units.length === 2) cancel = true;
        return Promise.resolve();
      },
    };
    const outcome = await runSteps(pass, { progress: () => undefined, frame: () => undefined }, () => cancel, pausing);
    expect(outcome).toEqual({ kind: 'cancelled' });
    expect(pass.units).toHaveLength(2);
    expect(pass.freed).toBe(1);
    expect(pass.finished).toBe(0);
  });

  it('frees the pass and throws on when the engine fails', async () => {
    const clock = fakeClock();
    const failure = new Error('the engine stopped');
    let calls = 0;
    const pass = {
      freed: 0,
      advance(): Step {
        calls += 1;
        if (calls === 2) throw failure;
        return { done: false, stage: 'evidence', fraction: 0, frames: [] };
      },
      finish: () => 'never',
      free() {
        pass.freed += 1;
      },
    };
    await expect(runSteps(pass, { progress: () => undefined, frame: () => undefined }, () => false, clock)).rejects.toBe(failure);
    expect(pass.freed).toBe(1);
  });
});
