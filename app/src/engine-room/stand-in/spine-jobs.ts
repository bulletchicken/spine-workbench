// How the stand-in's spine worker answers each spine job: the chain pass
// plays the story's chain frames for the films it is given (each the story
// film its strip names), the checks are worked out from the case, a saved
// case reads back exactly, and the made-up spine is the story's.

import type { FilmKind, Stage } from '@/shapes';
import type { StandInStory } from '@/sample-case/story-shape';
import { EngineFailure } from '../engine-client';
import { compareWithAnswerKey } from './answer-key';
import { packCase, unpackCase } from './case-bytes';
import { checkCase } from './checks';
import { pace, storyFilmOf } from './film-jobs';
import type { StandInScripts } from './stand-in-engine';
import type { StandInAnswer, StandInJobs } from './stand-in-worker';

export const CHAIN_STAGES: readonly Stage[] = ['chain', 'naming', 'confidence'];

export function spineJobs(story: StandInStory, scripts: StandInScripts): StandInJobs<null> {
  const answer = <Output>(delayMs: number, value: () => Output): StandInAnswer<Output> => ({ kind: 'answer', delayMs, answer: value });
  const { films, find, adjust, save, reopen } = scripts;

  return {
    'chain-pass': ({ results }) => {
      const { frames: all, marks, twoFilms } = story.pass;
      // Each slot, and the story film its film pass ran on.
      const slots = results.map((result) => ({ slot: result.film, told: storyFilmOf(result.strip) ?? result.film }));
      const told = (kind: FilmKind) => {
        const found = marks.find((each) => each.film === kind);
        if (found === undefined) throw new EngineFailure({ code: 'pass-no-spine-found', message: 'the story has no marks for this film', detail: kind });
        return found;
      };
      for (const { told: kind } of slots) told(kind);
      // A story film two slots answer as plays its frames into both.
      const frames = all.flatMap((frame) =>
        CHAIN_STAGES.includes(frame.stage) ? slots.filter((each) => each.told === frame.film).map(({ slot }) => ({ ...frame, film: slot })) : [],
      );
      const kinds = slots.map((each) => each.slot);
      return {
        kind: 'pass',
        frames,
        gapMs: (pace(scripts) * find.chainShare) / Math.max(1, frames.length),
        answer: () => ({
          films: slots.map(({ slot, told: kind }) => ({ film: slot, naming: { ...told(kind).naming, l5ConfirmedOn: null }, chain: new Uint8Array(0), warnings: [] })),
          twoFilms: kinds.includes('front') && kinds.includes('side') ? twoFilms : null,
        }),
      };
    },
    'check-case': (input) => answer(adjust.checkMs, () => checkCase(input)),
    'pack-case': (input) => answer(save.packMs, () => packCase(input)),
    'unpack-case': ({ bytes }) =>
      answer(reopen.unpackMs, () => {
        if (sameBytes(new Uint8Array(bytes), new Uint8Array(story.refusedCase.file))) throw new EngineFailure(story.refusedCase.error);
        return unpackCase(bytes);
      }),
    'make-up-spine': () => answer(films.makeUpMs, () => story.madeUp.spine),
    'compare-with-answer-key': ({ marks, key }) => answer(adjust.compareMs, () => compareWithAnswerKey(marks, key)),
  };
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, index) => byte === b[index]);
}
