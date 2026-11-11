// The stand-in telling the sample case's story, the one it tells in
// development: the made-up spine W1-09 opens, the case PNGs a save
// writes, and the pass at the slow pace W1-25 shows.

import story from '@/sample-case/story';
import type { CaseFileV1, Film, FilmKind } from '@/shapes';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { EngineFailure } from '../engine-client';
import type { RealEngine } from '../crew/real-engine';
import { DEFAULT_SCRIPTS, startStandInEngine, type StandInScripts } from './stand-in-engine';

const KINDS: FilmKind[] = ['front', 'side', 'bend-left', 'bend-right'];
const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

const QUICK: Partial<StandInScripts> = {
  films: { openMs: 0, displayPixelsMs: 0, makeUpMs: 0 },
  sizeAndFacing: { ...DEFAULT_SCRIPTS.sizeAndFacing, senseMs: 0, scaleMs: 0 },
  find: { ...DEFAULT_SCRIPTS.find, normalMs: 40, placePointsMs: 0 },
  save: { encodeMs: 0, packMs: 0 },
  reopen: { unpackMs: 0 },
};

let engine: RealEngine | null = null;
function start(scripts: Partial<StandInScripts> = QUICK): RealEngine {
  engine = startStandInEngine({ story, scripts });
  return engine;
}
afterEach(() => {
  engine?.stop();
  engine = null;
  vi.useRealTimers();
});

async function codeOf(promise: Promise<unknown>): Promise<string> {
  try {
    await promise;
    return 'answered';
  } catch (thrown) {
    return thrown instanceof EngineFailure ? thrown.error.code : String(thrown);
  }
}

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

describe('the stand-in telling the sample case', () => {
  it('makes up the sample case’s spine: its films as PNGs and its answer key', async () => {
    const spine = await start().makeUpSpine({ name: 'straight', seed: 1, tier: 't0' });
    expect(Object.keys(spine).sort()).toEqual(['answerKey', 'films']);
    expect(spine.films.map((film) => film.kind)).toEqual(KINDS);
    expect(spine.answerKey.films).toHaveLength(4);
  });

  it('opens each film the made-up spine hands over, with the made-up spine’s facts and guess', async () => {
    const client = start();
    for (const { kind, png } of story.madeUp.spine.films) {
      const opened = await client.openFilm(kind, png.slice().buffer, null);
      expect(opened.facts).toEqual(story.madeUp.films[kind].facts);
      expect(await client.senseGeometry(kind)).toEqual(story.madeUp.films[kind].guess);
    }
  });

  it('writes each film into a saved case as a PNG, which opens again', async () => {
    const client = start();
    for (const kind of KINDS) await client.openFilm(kind, story.films[kind].file, null);
    const c: CaseFileV1 = { format: 'spine-case', version: 1, savedWith: { app: 'test' }, films: KINDS.map(recordOf), marks: [], nextVertebraId: 1 };
    const saved = await client.saveCase(c);
    const opened = await client.openCase(saved.slice().buffer);
    for (const film of opened.films) {
      expect([...film.png.subarray(0, 8)]).toEqual(PNG_SIGNATURE);
      expect(await codeOf(client.openFilm(film.kind, film.png.slice().buffer, recordOf(film.kind)))).toBe('answered');
    }
  });

  it('runs the pass at the slow pace to its answer, with three film-stage frames a film', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
    const client = start({ ...QUICK, find: { ...DEFAULT_SCRIPTS.find, placePointsMs: 0 }, pace: 'slow' });
    const opening = client.openFilm('front', story.films.front.file, null);
    await vi.advanceTimersByTimeAsync(10);
    await opening;
    let ended = false;
    const passing = codeOf(client.runAutoPass({ caseVersion: 1, films: [recordOf('front')], options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId: 1 })).finally(() => {
      ended = true;
    });
    // The slow pace's whole pass is 24 s: still running at 23.5 s, answered by 25 s.
    await vi.advanceTimersByTimeAsync(23_500);
    expect(ended, 'still running half a second before the scripted 24 s').toBe(false);
    await vi.advanceTimersByTimeAsync(1_500);
    expect(await passing).toBe('answered');
  });
});
