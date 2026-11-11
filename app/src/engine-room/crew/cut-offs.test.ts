// The same rules, held to the same tests, for both engines the screens
// can be handed: the real engine's crew over engine workers (here the
// real worker job loop with fake jobs), and the stand-in. Each rule is
// the contract's wording in docs/contracts/engine-api.md and
// worker-messages.md.

import type { AutoPassInput, Film, FilmKind, FilmMarks, ToolPreviewInput, WatchFrame } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { smallStory } from '../stand-in/small-story';
import { FILM_STAGES } from '../stand-in/film-jobs';
import { DEFAULT_SCRIPTS, startStandInEngine } from '../stand-in/stand-in-engine';
import type { RealEngine } from './real-engine';
import { fakeCrew, fakePassFrames, fakePassSlices, filmFile, outcome, settle } from './test-rig';

/**
 * The frames the contract lets a worker thin, written out here rather than
 * taken from the step loop so this test holds both engines to the
 * contract on its own (docs/contracts/worker-messages.md, rule 7):
 * `joint-progress`, and `evidence` that is not final and not the wavefront.
 */
function mayBeThinned(frame: WatchFrame): boolean {
  if (frame.kind === 'joint-progress') return true;
  return frame.kind === 'evidence' && !frame.final && frame.map !== 'wavefront';
}

interface Subject {
  name: string;
  start(): RealEngine;
  /** Two films for the front slot, one that cannot be read, and one for the side. */
  files: { first: ArrayBuffer; second: ArrayBuffer; unreadable: ArrayBuffer; side: ArrayBuffer };
  /** How many pixels each front film has, which tells them apart. */
  pixels: { first: number; second: number };
  /** Bytes the engine reads as a saved case. */
  savedCase(): Promise<ArrayBuffer>;
  /** A front and a side film whose passes mix frames that may be thinned with keyframes. */
  mixed: { front: ArrayBuffer; side: ArrayBuffer };
  /** Every frame a pass over those two films makes, job by job: each film's film pass, and the chain pass. */
  emitted(): Record<'front' | 'side' | 'chain', WatchFrame[]>;
}

/** Which job made a frame: the film pass of its film, or the chain pass. */
const jobOf = (frame: WatchFrame): 'front' | 'side' | 'chain' => (FILM_STAGES.includes(frame.stage) ? (frame.film as 'front' | 'side') : 'chain');

/** The frames of a fake pass over a film of this name. */
const fakeFrames = (film: FilmKind, stage: 'evidence' | 'chain', name: string, quick: number) =>
  Array.from({ length: fakePassSlices(name, quick) }, (_, slice) => fakePassFrames(film, stage, slice + 1, name.includes('mixed'))).flat();

const story = smallStory();
const recordOf = (kind: FilmKind): Film => {
  const facts = story.films[kind].facts;
  return {
    kind,
    image: { path: `films/${kind}.png`, width: facts.width, height: facts.height, bitsStored: 8 },
    source: 'png',
    keptTags: null,
    values: facts.values,
    flags: [],
    blanked: [],
    geometry: { mmPerPx: null, mmPerPxVertical: null, scale: null, quarterTurns: 0, imageRight: null, confirmed: { upright: false, leftRight: false, facing: false, scale: false, polarity: false, privacy: false } },
  };
};
const twoFilmCase = { format: 'spine-case' as const, version: 1 as const, savedWith: { app: 'test' }, films: [recordOf('front'), recordOf('side')], marks: [], nextVertebraId: 0 };
const pass = (signal?: AbortSignal): [AutoPassInput, { onFrame(frame: WatchFrame): void; signal?: AbortSignal }, WatchFrame[]] => {
  const frames: WatchFrame[] = [];
  return [
    { caseVersion: 1, films: [recordOf('front'), recordOf('side')], options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId: 0 },
    { onFrame: (frame) => frames.push(frame), ...(signal === undefined ? {} : { signal }) },
    frames,
  ];
};
const preview = (caseVersion: number): ToolPreviewInput => ({ caseVersion, tool: 'boundary-wand', at: [60, 67], marks: story.pass.marks[0] as FilmMarks, newVertebraId: 12 });
const sliced = (bytes: Uint8Array) => bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;

/** Short pauses everywhere, so calls are still waiting or running when a test cuts them off. */
const PAUSED = {
  films: { openMs: 4, displayPixelsMs: 4, makeUpMs: 4 },
  sizeAndFacing: { ...DEFAULT_SCRIPTS.sizeAndFacing, senseMs: 4, scaleMs: 4 },
  find: { ...DEFAULT_SCRIPTS.find, normalMs: 300, placePointsMs: 4 },
  adjust: { ...DEFAULT_SCRIPTS.adjust, previewMs: 4, wandMs: 4, pinMs: 4, checkMs: 4, compareMs: 4 },
  save: { encodeMs: 4, packMs: 4 },
  reopen: { unpackMs: 4 },
};

const SUBJECTS: Subject[] = [
  {
    name: 'the crew over engine workers',
    start: () => fakeCrew().engine,
    files: { first: filmFile('front slow A'), second: filmFile('front slow BB'), unreadable: filmFile('bad bytes'), side: filmFile('side slow') },
    pixels: { first: 'front slow A'.length, second: 'front slow BB'.length },
    savedCase: () => Promise.resolve(filmFile('a case')),
    mixed: { front: filmFile('front mixed'), side: filmFile('side mixed') },
    emitted: () => ({
      front: fakeFrames('front', 'evidence', 'front mixed', 3),
      side: fakeFrames('side', 'evidence', 'side mixed', 3),
      chain: fakeFrames('front', 'chain', 'front mixed side mixed', 2),
    }),
  },
  {
    name: 'the stand-in',
    start: () => startStandInEngine({ story, scripts: PAUSED }),
    files: { first: story.films.front.file, second: story.films['bend-right'].file, unreadable: filmFile('not a film'), side: story.films.side.file },
    pixels: { first: 120 * 300, second: 116 * 300 },
    savedCase: async () => {
      const engine = startStandInEngine({ story, scripts: PAUSED });
      await engine.openFilm('front', story.films.front.file, null);
      await engine.openFilm('side', story.films.side.file, null);
      const saved = await engine.saveCase(twoFilmCase);
      engine.stop();
      return sliced(saved);
    },
    mixed: { front: story.films.front.file, side: story.films.side.file },
    emitted: () => ({
      front: story.pass.frames.filter((frame) => frame.film === 'front' && FILM_STAGES.includes(frame.stage)),
      side: story.pass.frames.filter((frame) => frame.film === 'side' && FILM_STAGES.includes(frame.stage)),
      chain: story.pass.frames.filter((frame) => (frame.film === 'front' || frame.film === 'side') && !FILM_STAGES.includes(frame.stage)),
    }),
  },
];

/** Whether `seen` is `made` with some frames left out and none reordered. */
function inOrderWithin(seen: WatchFrame[], made: WatchFrame[]): boolean {
  const text = made.map((frame) => JSON.stringify(frame));
  let at = 0;
  for (const frame of seen) {
    at = text.indexOf(JSON.stringify(frame), at);
    if (at < 0) return false;
    at += 1;
  }
  return true;
}

describe.each(SUBJECTS)('$name', (subject) => {
  const opened = async () => {
    const engine = subject.start();
    await engine.openFilm('front', subject.files.first, null);
    return engine;
  };
  const pixelCount = async (engine: RealEngine) => (await engine.displayPixels('front')).length;

  it('rejects every call waiting or running on a replaced film; a call made after reaches the new one', async () => {
    const engine = await opened();
    const running = engine.displayPixels('front');
    const waiting = engine.senseGeometry('front');
    const replacing = engine.openFilm('front', subject.files.second, null);
    const after = pixelCount(engine);
    expect(await outcome(running)).toBe('pass-cancelled');
    expect(await outcome(waiting)).toBe('pass-cancelled');
    expect(await outcome(replacing)).toBe('answered');
    expect(await after).toBe(subject.pixels.second);
    engine.stop();
  });

  it('keeps the open film open and unchanged when the replacing film cannot be read', async () => {
    const engine = await opened();
    const cutOff = engine.senseGeometry('front');
    const refused = engine.openFilm('front', subject.files.unreadable, null);
    expect(await outcome(cutOff)).toBe('pass-cancelled');
    expect(await outcome(refused)).toBe('film-unknown-format');
    expect(await pixelCount(engine)).toBe(subject.pixels.first);
    engine.stop();
  });

  it('rejects an openFilm still opening when its kind is opened again', async () => {
    const engine = subject.start();
    const first = engine.openFilm('front', subject.files.first, null);
    await settle(2);
    const second = engine.openFilm('front', subject.files.second, null);
    expect(await outcome(first)).toBe('pass-cancelled');
    expect(await outcome(second)).toBe('answered');
    expect(await pixelCount(engine)).toBe(subject.pixels.second);
    engine.stop();
  });

  it('rejects the calls on a retired film and refuses later ones until it is opened again', async () => {
    const engine = await opened();
    const running = engine.displayPixels('front');
    const waiting = engine.senseGeometry('front');
    engine.retireFilm('front');
    expect(await outcome(running)).toBe('pass-cancelled');
    expect(await outcome(waiting)).toBe('pass-cancelled');
    expect(await outcome(engine.displayPixels('front'))).toBe('engine-out-of-sync');
    await engine.openFilm('front', subject.files.second, null);
    expect(await pixelCount(engine)).toBe(subject.pixels.second);
    engine.stop();
  });

  it('rejects a replaceable call that a newer one replaces while it waits', async () => {
    const engine = await opened();
    const first = engine.previewTool('front', preview(1));
    const second = engine.previewTool('front', preview(2));
    const third = engine.previewTool('front', preview(3));
    expect(await outcome(second)).toBe('pass-cancelled');
    expect(await first).toMatchObject({ caseVersion: 1 });
    expect(await third).toMatchObject({ caseVersion: 3 });
    engine.stop();
  });

  it('rejects a whole pass when one of its films is replaced mid-pass, with no frame after', async () => {
    const engine = await opened();
    await engine.openFilm('side', subject.files.side, null);
    const [input, run, frames] = pass();
    const running = engine.runAutoPass(input, run);
    let seen = -1;
    running.catch(() => {
      seen = frames.length;
    });
    await settle(6);
    void engine.openFilm('side', subject.files.side, null);
    expect(await outcome(running)).toBe('pass-cancelled');
    await settle();
    expect(frames.length).toBe(seen);
    // Calls on the other film are untouched.
    expect(await pixelCount(engine)).toBe(subject.pixels.first);
    engine.stop();
  });

  it('rejects a whole pass when one of its films is retired, and when its signal aborts', async () => {
    const engine = await opened();
    await engine.openFilm('side', subject.files.side, null);
    const [input, run] = pass();
    const retired = engine.runAutoPass(input, run);
    await settle(6);
    engine.retireFilm('side');
    expect(await outcome(retired)).toBe('pass-cancelled');

    await engine.openFilm('side', subject.files.side, null);
    const controller = new AbortController();
    const [again, aborting, frames] = pass(controller.signal);
    const aborted = engine.runAutoPass(again, aborting);
    await settle(6);
    controller.abort();
    const seen = frames.length;
    expect(await outcome(aborted)).toBe('pass-cancelled');
    await settle();
    expect(frames).toHaveLength(seen);
    engine.stop();
  });

  it('rejects a whole save when one of its films is replaced', async () => {
    const engine = await opened();
    await engine.openFilm('side', subject.files.side, null);
    const saving = engine.saveCase(twoFilmCase);
    void engine.openFilm('side', subject.files.side, null);
    expect(await outcome(saving)).toBe('pass-cancelled');
    expect(await outcome(engine.saveCase(twoFilmCase))).toBe('answered');
    engine.stop();
  });

  it('opens no film when it opens a case', async () => {
    const bytes = await subject.savedCase();
    const engine = subject.start();
    expect(await outcome(engine.openCase(bytes))).toBe('answered');
    expect(await outcome(engine.displayPixels('front'))).toBe('engine-out-of-sync');
    engine.stop();
  });

  it('thins only the frames that may be thinned, keeps each job’s frames in the order made, and sends none after a stop', async () => {
    const engine = subject.start();
    await engine.openFilm('front', subject.mixed.front, null);
    await engine.openFilm('side', subject.mixed.side, null);
    const [input, run, frames] = pass();
    await engine.runAutoPass(input, run);
    const made = subject.emitted();
    for (const job of ['front', 'side', 'chain'] as const) {
      const seen = frames.filter((frame) => jobOf(frame) === job);
      expect(made[job].some(mayBeThinned), `${job} makes frames that may be thinned`).toBe(true);
      expect(seen.filter((frame) => !mayBeThinned(frame)), `every keyframe of the ${job} job`).toEqual(made[job].filter((frame) => !mayBeThinned(frame)));
      expect(inOrderWithin(seen, made[job]), `the ${job} job's order`).toBe(true);
    }

    const controller = new AbortController();
    const [again, stopping, stopped] = pass(controller.signal);
    const aborted = engine.runAutoPass(again, stopping);
    await settle(6);
    controller.abort();
    const sent = stopped.length;
    expect(await outcome(aborted)).toBe('pass-cancelled');
    await settle();
    expect(stopped).toHaveLength(sent);
    for (const job of ['front', 'side', 'chain'] as const) expect(inOrderWithin(stopped.filter((frame) => jobOf(frame) === job), made[job])).toBe(true);
    engine.stop();
  });
});
