// The stand-in engine: dev and tests only, never shipped. It is the real
// engine's crew with stand-in workers in place of the engine's, so every
// rule about queues, replaceable calls, films replaced or retired and
// passes stopped mid-way holds for it exactly as it holds for the engine.
// What it answers comes from a story (the sample case's, or its own small
// one), and how long it takes and how its tools behave come from its
// scripts, one per step.

import type { StandInStory } from '@/sample-case/story-shape';
import { Crew } from '../crew/crew';
import { RealEngine } from '../crew/real-engine';
import { filmJobs, type FilmRoom } from './film-jobs';
import adjust, { type AdjustScript } from './scripts/adjust';
import films, { type FilmsScript } from './scripts/films';
import find, { type FindScript } from './scripts/find';
import reopen, { type ReopenScript } from './scripts/reopen';
import save, { type SaveScript } from './scripts/save';
import sizeAndFacing, { type SizeAndFacingScript } from './scripts/size-and-facing';
import { smallStory } from './small-story';
import { spineJobs } from './spine-jobs';
import { StandInWorker } from './stand-in-worker';

export interface StandInScripts {
  films: FilmsScript;
  sizeAndFacing: SizeAndFacingScript;
  find: FindScript;
  adjust: AdjustScript;
  save: SaveScript;
  reopen: ReopenScript;
  /** The automatic pass at its normal pace, or at its slow one. */
  pace: 'normal' | 'slow';
}

export const DEFAULT_SCRIPTS: StandInScripts = { films, sizeAndFacing, find, adjust, save, reopen, pace: 'normal' };

export interface StandInSetup {
  /** The story it tells; its own small one when left out. */
  story?: StandInStory | null;
  /** Scripts to use in place of the defaults, step by step. */
  scripts?: Partial<StandInScripts>;
}

/** The compiled engine the crew hands each worker at boot; a stand-in worker never uses it. */
const NO_ENGINE = new WebAssembly.Module(new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]));

export function startStandInEngine(setup: StandInSetup = {}): RealEngine {
  const story = setup.story ?? smallStory();
  const scripts: StandInScripts = { ...DEFAULT_SCRIPTS, ...setup.scripts };
  const forFilms = filmJobs(story, scripts);
  const forSpine = spineJobs(story, scripts);
  return new RealEngine(
    new Crew({
      spawn: (role, events) =>
        role === 'film' ? new StandInWorker<FilmRoom>(role, events, forFilms, { film: null }) : new StandInWorker<null>(role, events, forSpine, null),
      engine: () => Promise.resolve(NO_ENGINE),
    }),
  );
}
