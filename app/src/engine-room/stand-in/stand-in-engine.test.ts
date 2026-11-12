import type { CaseFileV1, Film, FilmKind, FilmMarks, Level, WatchFrame } from '@/shapes';
import { CASE_FORMAT, CASE_VERSION, LEVELS, SHAPES_FINGERPRINT } from '@/shapes';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { EngineFailure } from '../engine-client';
import type { RealEngine } from '../crew/real-engine';
import { smallStory } from './small-story';
import { DEFAULT_SCRIPTS, startStandInEngine, type StandInScripts } from './stand-in-engine';

const KINDS: FilmKind[] = ['front', 'side', 'bend-left', 'bend-right'];
const story = smallStory();

/** Every delay zero and the whole pass 40 ms, so tests run quickly. */
const QUICK: Partial<StandInScripts> = {
  films: { openMs: 0, displayPixelsMs: 0, makeUpMs: 0 },
  sizeAndFacing: { ...DEFAULT_SCRIPTS.sizeAndFacing, senseMs: 0, scaleMs: 0 },
  find: { ...DEFAULT_SCRIPTS.find, normalMs: 40, placePointsMs: 0 },
  adjust: { ...DEFAULT_SCRIPTS.adjust, previewMs: 0, wandMs: 0, pinMs: 0, checkMs: 0, compareMs: 0 },
  save: { encodeMs: 0, packMs: 0 },
  reopen: { unpackMs: 0 },
};

let engine: RealEngine | null = null;
function start(): RealEngine {
  engine = startStandInEngine({ story, scripts: QUICK });
  return engine;
}
afterEach(() => {
  engine?.stop();
  engine = null;
});

async function codeOf(promise: Promise<unknown>): Promise<string> {
  try {
    await promise;
    return 'answered';
  } catch (thrown) {
    return thrown instanceof EngineFailure ? thrown.error.code : String(thrown);
  }
}

/** The film record the case would hold for a story film. */
function recordOf(kind: FilmKind): Film {
  const facts = story.films[kind].facts;
  return {
    kind,
    image: { path: `films/${kind}.png`, width: facts.width, height: facts.height, bitsStored: facts.bitsStored },
    source: facts.source,
    keptTags: facts.keptTags,
    values: facts.values,
    flags: facts.flags,
    blanked: [],
    geometry: { mmPerPx: null, mmPerPxVertical: null, scale: null, quarterTurns: 0, imageRight: null, confirmed: { upright: false, leftRight: false, facing: false, scale: false, polarity: false, privacy: false } },
  };
}

async function openAll(client: RealEngine): Promise<void> {
  for (const kind of KINDS) await client.openFilm(kind, story.films[kind].file, null);
}

describe('the stand-in engine', () => {
  it('reports itself as built from the app’s shapes', async () => {
    expect(await start().info()).toEqual({ engineVersion: 'stand-in', shapesFingerprint: SHAPES_FINGERPRINT, caseFormat: CASE_FORMAT, caseVersion: CASE_VERSION });
  });

  it('opens a story film by its bytes, and refuses anything else by its bytes', async () => {
    const client = start();
    const opened = await client.openFilm('front', story.films.front.file, null);
    expect(opened.facts).toEqual(story.films.front.facts);
    expect(opened.thumbnail).toEqual(story.films.front.thumbnail);
    expect(await codeOf(client.openFilm('side', new TextEncoder().encode('not a film').buffer, null))).toBe('film-unknown-format');
  });

  it('gives each film’s pixels as half floats of its drawn values', async () => {
    const client = start();
    await client.openFilm('front', story.films.front.file, null);
    const pixels = await client.displayPixels('front');
    const { width, height } = story.films.front.facts;
    expect(pixels).toHaveLength(width * height);
    // (60, 67) is inside L3's body (value 200), (2, 2) is bare film (25), (60, 2) soft tissue (70).
    // Their half floats, as IEEE 754 binary16 bits: 200/255 is 2⁻¹ × (1 + 582/1024), 0x3A46;
    // 25/255 is 2⁻⁴ × (1 + 582/1024), 0x2E46; 70/255 is 2⁻² × (1 + 100/1024), 0x3464.
    expect(pixels[67 * width + 60]).toBe(0x3a46);
    expect(pixels[2 * width + 2]).toBe(0x2e46);
    expect(pixels[2 * width + 60]).toBe(0x3464);
  });

  it('gives the story’s guess for each film', async () => {
    const client = start();
    await openAll(client);
    for (const kind of KINDS) expect(await client.senseGeometry(kind)).toEqual(story.films[kind].guess);
  });

  it('works a ruler’s scale out as its length over its pixels, and refuses one too short or of no length', async () => {
    const client = start();
    await client.openFilm('side', story.films.side.file, null);
    // 100 px for 50 mm: 0.5 mm per pixel, weak below 150 px, uncertain by 1.5·√2 px in 100.
    expect(await client.scaleFromRuler('side', [10, 10], [10, 110], 50)).toEqual({
      mmPerPx: 0.5,
      mmPerPxVertical: null,
      source: { source: 'ruler', rulers: [{ a: [10, 10], b: [10, 110], mm: 50 }] },
      plane: 'unknown',
      relativeUncertainty: (1.5 * Math.SQRT2) / 100,
      weak: true,
    });
    expect(await codeOf(client.scaleFromRuler('side', [10, 10], [16, 18], 50))).toBe('scale-too-short');
    expect(await codeOf(client.scaleFromRuler('side', [10, 10], [10, 110], 0))).toBe('scale-bad-length');
  });

  it('works a round object’s scale out from where it was pressed and let go', async () => {
    const client = start();
    await client.openFilm('bend-right', story.films['bend-right'].file, null);
    // A 3-4-5 triangle times ten: a radius of 50 px for a 25 mm ball, 0.25 mm per pixel.
    expect(await client.scaleFromKnownObject('bend-right', [50, 50], [80, 90], 25)).toMatchObject({
      mmPerPx: 0.25,
      source: { source: 'known-object', centre: [50, 50], radiusPx: 50, diameterMm: 25 },
      plane: 'object-depth',
      weak: true,
    });
  });

  it('runs the pass: the story’s frames in order, its marks with ids counting from the first free one', async () => {
    const client = start();
    await openAll(client);
    const frames: WatchFrame[] = [];
    const progress: string[] = [];
    const result = await client.runAutoPass(
      { caseVersion: 12, films: KINDS.map(recordOf), options: { acquisition: 'unknown', population: 'adolescent', template: null }, firstVertebraId: 100, frontFinder: 'hand-built' },
      { onFrame: (frame) => frames.push(frame), onProgress: ({ stage }) => progress.push(stage) },
    );
    expect(result.caseVersion).toBe(12);
    expect(result.twoFilms).toEqual(story.pass.twoFilms);
    expect(result.warnings).toEqual(story.pass.warnings);
    expect(result.marks.map((marks) => [marks.film, marks.vertebrae.map((vertebra) => vertebra.id)])).toEqual([
      ['front', [100, 101, 102]],
      ['side', [103, 104, 105]],
      ['bend-left', [106, 107, 108]],
      ['bend-right', [109, 110, 111]],
    ]);
    expect(result.marks.map((marks) => ({ ...marks, vertebrae: marks.vertebrae.map((vertebra) => ({ ...vertebra, id: vertebra.id - 100 })) }))).toEqual(story.pass.marks);
    // Each film's frames arrive in the order the story gives them: every keyframe, and
    // the frames the step loop may thin only as a part of them, never out of order.
    for (const kind of KINDS) {
      const told = story.pass.frames.filter((frame) => frame.film === kind).map((frame) => JSON.stringify(frame));
      const seen = frames.filter((frame) => frame.film === kind).map((frame) => JSON.stringify(frame));
      const keyframes = story.pass.frames.filter((frame) => frame.film === kind && frame.kind !== 'joint-progress' && !(frame.kind === 'evidence' && !frame.final));
      expect(keyframes.every((frame) => seen.includes(JSON.stringify(frame)))).toBe(true);
      expect(seen.every((frame, index) => index === 0 || told.indexOf(frame) > told.indexOf(seen[index - 1] as string))).toBe(true);
    }
    expect(progress.length).toBeGreaterThan(0);
  });

  it('runs a pass over two films with no two-film check unless front and side are both in it', async () => {
    const client = start();
    await openAll(client);
    const result = await client.runAutoPass({ caseVersion: 1, films: [recordOf('front'), recordOf('bend-left')], options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId: 0, frontFinder: 'hand-built' });
    expect(result.twoFilms).toBeNull();
    expect(result.marks.map((marks) => marks.vertebrae.map((vertebra) => vertebra.id))).toEqual([
      [0, 1, 2],
      [3, 4, 5],
    ]);
  });

  it('answers the tools from the marks it is given, echoing the case version', async () => {
    const client = start();
    await openAll(client);
    const front = story.pass.marks[0] as FilmMarks;
    // L4 on the front film runs from y 110 to 164 near x 60: its middle splits it.
    const preview = await client.previewTool('front', { caseVersion: 5, tool: 'boundary-wand', at: [60, 137], marks: front, newVertebraId: 12 });
    expect(preview).toMatchObject({ action: 'split', caseVersion: 5, vertebra: 1, levelsAfter: [{ vertebra: 12, level: 'L5' }, { vertebra: 2, level: 'L6' }] });
    const split = await client.boundaryWand('front', { caseVersion: 5, marks: front, at: [60, 137], newVertebraId: 12 });
    expect(split.marks.vertebrae.map((vertebra) => [vertebra.id, vertebra.level])).toEqual([
      [0, 'L3'],
      [1, 'L4'],
      [12, 'L5'],
      [2, 'L6'],
    ]);
    const grown = await client.regionWand('front', { caseVersion: 6, marks: front, vertebra: 0, clicks: [{ at: [90, 67], polarity: 'add' }] });
    expect(grown).toMatchObject({ caseVersion: 6, changed: [0] });
    const pinned = await client.resolvePins('side', { caseVersion: 7, marks: story.pass.marks[1] as FilmMarks, action: { action: 'move', target: { on: 'vertebra', vertebra: 4, mark: 'infA' }, to: [86, 166] } });
    expect(pinned).toMatchObject({ caseVersion: 7, changed: [4, 5] });
    expect(await client.previewTool('bend-left', { caseVersion: 8, tool: 'region-wand', at: [60, 67], marks: story.pass.marks[2] as FilmMarks, newVertebraId: 12 })).toEqual({
      action: 'none',
      caseVersion: 8,
      reason: 'not-supported-on-film',
    });
  });

  it('checks the case it is given', async () => {
    const client = start();
    const c: CaseFileV1 = { format: 'spine-case', version: 1, savedWith: { app: 'test' }, films: [recordOf('front')], marks: [story.pass.marks[0] as FilmMarks], nextVertebraId: 3 };
    const findings = await client.checkCase(c);
    const missing = findings.filter((finding) => finding.kind === 'vertebra-missing').map((finding) => finding.level);
    expect(missing).toEqual(LEVELS.slice(LEVELS.indexOf('T1'), LEVELS.indexOf('L3')).filter((level: Level) => level !== 'T13'));
    expect(findings.filter((finding) => finding.kind !== 'vertebra-missing').map((finding) => finding.kind)).toEqual([
      'l5-unconfirmed',
      // The film's facing is not known yet, so its marks rest on a guess.
      'marks-named-for-other-facing',
      'upright-unconfirmed',
      'left-right-unconfirmed',
      'scale-unconfirmed',
      'polarity-unconfirmed',
      'privacy-unchecked',
      'pedicle-not-visible',
    ]);
  });

  it('saves a case and opens it again exactly, opening no film', async () => {
    const client = start();
    await openAll(client);
    const c: CaseFileV1 = { format: 'spine-case', version: 1, savedWith: { app: 'test' }, films: KINDS.map(recordOf), marks: story.pass.marks, nextVertebraId: 12 };
    const saved = await client.saveCase(c);
    const reopened = startStandInEngine({ story, scripts: QUICK });
    try {
      const opened = await reopened.openCase(saved.buffer.slice(saved.byteOffset, saved.byteOffset + saved.byteLength) as ArrayBuffer);
      expect(opened.case).toEqual(c);
      expect(opened.films.map((film) => film.kind)).toEqual(KINDS);
      expect(await codeOf(reopened.displayPixels('front'))).toBe('engine-out-of-sync');
      // Each film opens again from its PNG and its saved record.
      for (const film of opened.films) {
        const png = film.png.buffer.slice(film.png.byteOffset, film.png.byteOffset + film.png.byteLength) as ArrayBuffer;
        expect((await reopened.openFilm(film.kind, png, recordOf(film.kind))).facts).toEqual(story.films[film.kind].facts);
      }
      // Each film's PNG is the made-up spine's PNG of that film, never the file it came in.
      const pngs = story.madeUp.spine.films;
      for (const film of opened.films) expect(film.png).toEqual(pngs.find((each) => each.kind === film.kind)?.png);
    } finally {
      reopened.stop();
    }
  });

  it('refuses the story’s case that will not open with the story’s error, and anything else as not a zip', async () => {
    const client = start();
    await expect(client.openCase(story.refusedCase.file)).rejects.toMatchObject({ error: story.refusedCase.error });
    expect(await codeOf(client.openCase(new TextEncoder().encode('{"not":"ours"}').buffer))).toBe('case-not-a-zip');
  });

  it('makes up the story’s spine, and scores marks against its answer key', async () => {
    const client = start();
    const spine = await client.makeUpSpine({ name: 'straight', seed: 1, tier: 't0' });
    expect(Object.keys(spine).sort()).toEqual(['answerKey', 'films']);
    expect(spine).toEqual(story.madeUp.spine);
    const comparison = await client.compareWithAnswerKey(story.pass.marks, spine.answerKey);
    // Every true corner lies 3 px right of and 4 px below its mark: 5 px, 2.5 mm at 0.5 mm per pixel.
    for (const film of comparison.films) {
      expect(film).toMatchObject({ expected: 3, found: 3, levelsRight: 3, meanCornerErrorPx: 5, meanCornerErrorMm: 2.5 });
      expect(film.vertebrae.map((vertebra) => vertebra.maxCornerErrorPx)).toEqual([5, 5, 5]);
    }
  });

  it('opens each of the made-up spine’s PNGs as its film, with the made-up spine’s facts and guess', async () => {
    const client = start();
    for (const { kind, png } of story.madeUp.spine.films) {
      const opened = await client.openFilm(kind, png.slice().buffer, null);
      expect(opened.facts).toEqual(story.madeUp.films[kind].facts);
      expect(opened.thumbnail).toEqual(story.films[kind].thumbnail);
      expect(await client.senseGeometry(kind)).toEqual(story.madeUp.films[kind].guess);
      expect(await client.displayPixels(kind)).toEqual(story.films[kind].pixels());
    }
  });

  it('answers a film of another view as the slot’s own story film, keeping the pixels of the film it was handed', async () => {
    const client = start();
    // The side film's file opened as the front film and the other way round.
    const opened = await client.openFilm('front', story.films.side.file, null);
    await client.openFilm('side', story.films.front.file, null);
    expect(opened.thumbnail).toEqual(story.films.side.thumbnail);
    expect(await client.displayPixels('front')).toEqual(story.films.side.pixels());
    expect(await client.senseGeometry('front')).toEqual(story.films.front.guess);
    expect(await client.senseGeometry('side')).toEqual(story.films.side.guess);
    const frames: WatchFrame[] = [];
    const result = await client.runAutoPass(
      { caseVersion: 1, films: [recordOf('front'), recordOf('side')], options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId: 0, frontFinder: 'hand-built' },
      { onFrame: (frame) => frames.push(frame) },
    );
    // Each slot takes the marks its view holds: pedicles and patient left or right on the front film.
    expect(result.marks[0]).toMatchObject({ film: 'front', namedFor: 'patient-left' });
    expect(result.marks[0]?.vertebrae.map((vertebra) => vertebra.marks.kind)).toEqual(['front', 'front', 'front']);
    expect(result.marks[1]).toMatchObject({ film: 'side', namedFor: 'anterior' });
    expect(result.marks[1]?.vertebrae.map((vertebra) => vertebra.marks.kind)).toEqual(['side', 'side', 'side']);
    // And the frames are each slot's own: only the front film's chain is solved with joint progress.
    const lockedInFront = frames.find((frame) => frame.kind === 'chain-locked' && frame.film === 'front');
    const frontLocked = story.pass.frames.find((frame) => frame.kind === 'chain-locked' && frame.film === 'front');
    expect(lockedInFront).toEqual(frontLocked);
    expect(frames.some((frame) => frame.kind === 'joint-progress' && frame.film === 'front')).toBe(true);
    expect(frames.some((frame) => frame.kind === 'joint-progress' && frame.film === 'side')).toBe(false);
    // A save stores the film the bytes show: the front slot holds the side film's file, so the side film's PNG.
    const pngs = story.madeUp.spine.films;
    const saved = await client.saveCase({ format: 'spine-case', version: 1, savedWith: { app: 'test' }, films: [recordOf('front'), recordOf('side')], marks: result.marks, nextVertebraId: 6 });
    const reopened = await client.openCase(saved.slice().buffer);
    expect(reopened.films.find((film) => film.kind === 'front')?.png).toEqual(pngs.find((each) => each.kind === 'side')?.png);
    expect(reopened.films.find((film) => film.kind === 'side')?.png).toEqual(pngs.find((each) => each.kind === 'front')?.png);
    // The made-up spine's side PNG opened as the front film answers the made-up front film's guess.
    await client.openFilm('front', (pngs.find((each) => each.kind === 'side')?.png as Uint8Array).slice().buffer, null);
    expect(await client.senseGeometry('front')).toEqual(story.madeUp.films.front.guess);
    expect(story.madeUp.films.front.guess).not.toEqual(story.madeUp.films.side.guess);
  });

  it('plays a bending film opened into both bending slots into each slot, chain frames and all', async () => {
    const client = start();
    await client.openFilm('bend-left', story.films['bend-left'].file, null);
    await client.openFilm('bend-right', story.films['bend-left'].file, null);
    const frames: WatchFrame[] = [];
    await client.runAutoPass(
      { caseVersion: 1, films: [recordOf('bend-left'), recordOf('bend-right')], options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId: 0, frontFinder: 'hand-built' },
      { onFrame: (frame) => frames.push(frame) },
    );
    const told = (kind: WatchFrame['kind']) => story.pass.frames.find((frame) => frame.kind === kind && frame.film === 'bend-left');
    for (const slot of ['bend-left', 'bend-right'] as const) {
      expect(frames.find((frame) => frame.kind === 'chain-locked' && frame.film === slot)).toEqual({ ...told('chain-locked'), film: slot });
      expect(frames.find((frame) => frame.kind === 'levels-named' && frame.film === slot)).toEqual({ ...told('levels-named'), film: slot });
      expect(frames.find((frame) => frame.kind === 'outlines' && frame.film === slot)).toEqual({ ...told('outlines'), film: slot });
    }
  });

  it('answers a bending film opened into the other bending slot as the story film it holds', async () => {
    const client = start();
    await client.openFilm('bend-left', story.films['bend-right'].file, null);
    await client.openFilm('bend-right', story.films['bend-left'].file, null);
    expect(await client.displayPixels('bend-left')).toEqual(story.films['bend-right'].pixels());
    expect(await client.senseGeometry('bend-left')).toEqual(story.films['bend-right'].guess);
    expect(await client.senseGeometry('bend-right')).toEqual(story.films['bend-left'].guess);
    const result = await client.runAutoPass(
      { caseVersion: 1, films: [recordOf('bend-left'), recordOf('bend-right')], options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId: 0, frontFinder: 'hand-built' },
      {},
    );
    const told = (kind: FilmKind) => story.pass.marks.find((marks) => marks.film === kind) as FilmMarks;
    // The bend-left slot holds the bending right film's endplate lines, with ids counting up from 0.
    const atLeft = (told('bend-right').vertebrae).map((vertebra, index) => ({ ...vertebra, id: index }));
    expect(result.marks[0]?.film).toBe('bend-left');
    expect(result.marks[0]?.vertebrae).toEqual(atLeft);
    expect(result.marks[1]?.vertebrae.map((vertebra) => vertebra.marks)).toEqual(told('bend-left').vertebrae.map((vertebra) => vertebra.marks));
    expect(result.marks[1]?.vertebrae.every((vertebra) => vertebra.marks.kind === 'bending')).toBe(true);
  });
});

describe('the stand-in at the slow pace', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('keeps a pass of few frames alive through long gaps, so the crew never takes it for stuck', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
    // Three film-stage frames per film, as the sample case's pass has: at the slow pace's
    // 24 s, 60% of it over three frames leaves 4.8 s between them, past the crew's 2 s.
    const few = { ...story, pass: { ...story.pass, frames: story.pass.frames.filter((frame) => frame.kind === 'stage-started') } };
    const client = startStandInEngine({ story: few, scripts: { ...QUICK, find: { ...DEFAULT_SCRIPTS.find, placePointsMs: 0 }, pace: 'slow' } });
    engine = client;
    const opening = client.openFilm('front', story.films.front.file, null);
    await vi.advanceTimersByTimeAsync(10);
    await opening;
    const frames: WatchFrame[] = [];
    let ended = false;
    const passing = codeOf(
      client.runAutoPass(
        { caseVersion: 1, films: [recordOf('front')], options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId: 0, frontFinder: 'hand-built' },
        { onFrame: (frame) => frames.push(frame) },
      ),
    ).finally(() => {
      ended = true;
    });
    // The film pass takes 14.4 s and the chain pass 9.6 s, the whole 24 s, each answer one gap after its last frame.
    await vi.advanceTimersByTimeAsync(23_500);
    expect(ended, 'still running half a second before the scripted 24 s').toBe(false);
    await vi.advanceTimersByTimeAsync(1_500);
    expect(await passing).toBe('answered');
    expect(frames.map((frame) => frame.stage)).toEqual(['evidence', 'centre-line', 'strip', 'chain', 'naming', 'confidence']);
  });

  it('spends one gap on a pass of no frames before it answers', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
    // No frames at all: the film pass's 14.4 s and the chain pass's 9.6 s are each one gap.
    const none = { ...story, pass: { ...story.pass, frames: [] } };
    const client = startStandInEngine({ story: none, scripts: { ...QUICK, find: { ...DEFAULT_SCRIPTS.find, placePointsMs: 0 }, pace: 'slow' } });
    engine = client;
    const opening = client.openFilm('front', story.films.front.file, null);
    await vi.advanceTimersByTimeAsync(10);
    await opening;
    let ended = false;
    const passing = codeOf(
      client.runAutoPass({ caseVersion: 1, films: [recordOf('front')], options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId: 0, frontFinder: 'hand-built' }),
    ).finally(() => {
      ended = true;
    });
    await vi.advanceTimersByTimeAsync(23_500);
    expect(ended, 'still running half a second before the scripted 24 s').toBe(false);
    await vi.advanceTimersByTimeAsync(1_500);
    expect(await passing).toBe('answered');
  });
});
