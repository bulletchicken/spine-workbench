// The trained finder in the release browser build, run in Node as a worker
// runs it, gives exactly the native engine's answer.
//
// The film is the made-up front film the finder's known answers were
// recorded on (`straight`, seed 7, tier t2), made by the browser build
// itself and checked against the fingerprint those answers keep. It goes
// through the product's path: its 16-bit PNG opened by `FilmDesk.open`,
// then `start_trained_pass`, `advance` in uneven slices and `finish`.
// known-answers/trained-pass/front.json holds the native answer to the
// same film, record and first id, and
// engine/trained-finder/tests/finds_the_vertebrae_on_a_made_up_film.rs
// holds the native engine to that file character for character, so equal
// here means the same bits in both builds. The answer is also checked for its shape: a
// front film's marks the strict reader takes, names flagged as estimates,
// corners marked as the auto pass marks them.

import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { FilmDesk, initSync, make_up_spine, write_case_json } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import { emptyCase } from '../../app/src/case-state/empty-case';
import type { Film, FilmFacts, MadeUpRecipe, MadeUpSpine, Mark, Stage, Step, TrainedPassResult, WatchFrame } from '../../app/src/shapes';

const REPOSITORY = fileURLToPath(new URL('../..', import.meta.url));
const ENGINE = `${REPOSITORY}app/src/engine-room/wasm-build/wasm_bridge_bg.wasm`;

interface Recording {
  recipe: MadeUpRecipe;
  firstVertebraId: number;
  film: Film;
  result: TrainedPassResult;
}

const recording = JSON.parse(readFileSync(`${REPOSITORY}proving-ground/known-answers/trained-pass/front.json`, 'utf8')) as Recording;
const knownAnswers = JSON.parse(readFileSync(`${REPOSITORY}engine/trained-finder/tests/known-answers/manifest.json`, 'utf8')) as { filmSha256: string };

let png: Uint8Array;
let facts: FilmFacts;
let result: TrainedPassResult;
const steps: Step[] = [];

// Making up the case and one whole find take a few seconds in the release engine.
beforeAll(async () => {
  initSync({ module: await WebAssembly.compile(readFileSync(ENGINE)) });
  const front = (make_up_spine(recording.recipe) as MadeUpSpine).films.find((film) => film.kind === 'front');
  if (front === undefined) throw new Error('the made-up case has no front film');
  png = front.png;
  const desk = FilmDesk.open(png, 'front');
  try {
    facts = desk.facts() as FilmFacts;
    const pass = desk.start_trained_pass(recording.film, recording.firstVertebraId);
    let consumed = false;
    try {
      steps.push(pass.advance(0) as Step);
      for (let i = 0; ; i++) {
        const step = pass.advance([1, 7, 2, 31, 200][i % 5] ?? 1) as Step;
        steps.push(step);
        if (step.done) break;
      }
      consumed = true;
      result = pass.finish() as TrainedPassResult;
    } finally {
      if (!consumed) pass.free();
    }
  } finally {
    desk.free();
  }
}, 300_000);

describe('the trained finder in the browser build', () => {
  it('reads the film the known answers were recorded on, with the record the native run was given', () => {
    expect(createHash('sha256').update(png).digest('hex')).toBe(knownAnswers.filmSha256);
    expect(facts.bitsStored).toBe(16);
    const record: Film = {
      kind: facts.kind,
      image: { path: 'films/front.png', width: facts.width, height: facts.height, bitsStored: facts.bitsStored },
      source: facts.source,
      keptTags: facts.keptTags,
      values: facts.values,
      flags: facts.flags,
      blanked: [],
      geometry: {
        mmPerPx: null,
        mmPerPxVertical: null,
        scale: null,
        quarterTurns: facts.exifQuarterTurns ?? 0,
        imageRight: null,
        confirmed: { upright: false, leftRight: false, facing: false, scale: false, polarity: false, privacy: false },
      },
    };
    expect(record).toStrictEqual(recording.film);
  });

  it('gives exactly the native answer', () => {
    expect(result).toStrictEqual(recording.result);
  });

  it('answers a front film’s marks the strict reader takes, its names flagged as estimates', () => {
    const { marks } = result;
    expect(marks.film).toBe('front');
    expect(marks.naming).toStrictEqual({ template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: ['estimated'], l5ConfirmedOn: null });
    expect(marks.landmarks).toStrictEqual({ c7: null, sacrumTop: null, hips: [] });
    expect(marks.vertebrae.length).toBeGreaterThanOrEqual(12);
    expect(marks.vertebrae.length).toBeLessThanOrEqual(17);
    marks.vertebrae.forEach((vertebra, index) => {
      expect(vertebra.id).toBe(recording.firstVertebraId + index);
      expect(vertebra.outline).toBeNull();
      if (vertebra.marks.kind !== 'front') throw new Error(`vertebra ${String(vertebra.id)} is not marked as on a front film`);
      const { supL, supR, infL, infR, pedL, pedR } = vertebra.marks.points;
      const corners: Mark[] = [supL, supR, infL, infR];
      const inferred = corners.map((corner) => corner.inferred);
      expect(corners.every((corner) => corner.source === 'auto' && !corner.pinned)).toBe(true);
      expect(inferred.every((flag) => flag === inferred[0])).toBe(true);
      expect([pedL, pedR]).toStrictEqual([
        { seen: 'not-visible', source: 'auto' },
        { seen: 'not-visible', source: 'auto' },
      ]);
      const { confidence } = vertebra;
      expect(confidence.calibrated).toBe(false);
      expect(confidence.reasons).toStrictEqual(inferred[0] === true ? ['no-evidence', 'level-uncertain'] : ['level-uncertain']);
      // The band is the score's, by the bounds in engine/shapes/src/confidence.rs.
      expect(confidence.band).toBe(confidence.score >= 0.9 ? 'high' : confidence.score >= 0.6 ? 'medium' : 'low');
    });
    const nextVertebraId = recording.firstVertebraId + marks.vertebrae.length;
    expect(() => write_case_json({ ...emptyCase(), films: [recording.film], marks: [marks], nextVertebraId })).not.toThrow();
  });

  it('reports its stages in order, with the frames of the existing kinds, whatever the slices', () => {
    expect(steps[0]).toStrictEqual({ done: false, stage: 'evidence', fraction: 0, frames: [] });
    const order: Stage[] = ['evidence', 'chain', 'naming', 'confidence'];
    const stages = steps.map((step) => order.indexOf(step.stage));
    expect(stages.every((stage, index) => stage >= 0 && (index === 0 || stage >= (stages[index - 1] ?? 0)))).toBe(true);
    expect(steps.at(-1)).toMatchObject({ done: true, stage: 'confidence', fraction: 1 });
    const frames: WatchFrame[] = steps.flatMap((step) => step.frames);
    expect(frames.every((frame) => frame.film === 'front')).toBe(true);
    expect(frames.filter((frame) => frame.kind !== 'warning').map((frame) => (frame.kind === 'stage-started' ? `start ${frame.stage}` : frame.kind))).toStrictEqual([
      'start evidence',
      'start chain',
      'chain-locked',
      'start naming',
      'levels-named',
      'start confidence',
      'confidence',
      'outlines',
    ]);
    const outlines = frames.at(-1);
    expect(outlines?.kind === 'outlines' && outlines.final).toBe(true);
  });
});
