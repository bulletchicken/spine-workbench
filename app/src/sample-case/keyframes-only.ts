// A short, honest pass made from the marks alone: the frames that need no
// picture of the film, in the order the engine emits them. Each film's
// three film stages start, then on each film the chain stages: the chain
// locked in from the sacrum upward, the levels named, the confidence, and
// the final outlines.
//
// The watch-frame rule keeps every frame except `joint-progress` frames
// and unfinished `evidence` previews other than `wavefront` ones; none of
// these is either, so a step loop that thins by that rule passes them all.
// The stand-in plays these until the whole pass is recorded from the films.

import type { ChainBody, FilmMarks, OutlineQuad, Stage, WatchFrame } from '@/shapes';
import { quadOf } from './marks';

const FILM_STAGES: readonly Stage[] = ['evidence', 'centre-line', 'strip'];

function chainBodiesOf(marks: FilmMarks): ChainBody[] {
  return marks.vertebrae.map((vertebra) => {
    const [supA, supB, infB, infA] = quadOf(vertebra.marks);
    return {
      level: vertebra.level,
      sup: { a: supA.at, b: supB.at },
      inf: { a: infA.at, b: infB.at },
      inferred: [supA, supB, infB, infA].some((mark) => mark.inferred),
    };
  });
}

function chainFrames(marks: FilmMarks): WatchFrame[] {
  const film = marks.film;
  const quads: OutlineQuad[] = marks.vertebrae.map((vertebra) => ({
    level: vertebra.level,
    corners: quadOf(vertebra.marks).map((mark) => mark.at) as OutlineQuad['corners'],
  }));
  return [
    { kind: 'stage-started', film, stage: 'chain' },
    { kind: 'chain-locked', film, stage: 'chain', bodies: chainBodiesOf(marks).reverse() },
    { kind: 'stage-started', film, stage: 'naming' },
    { kind: 'levels-named', film, stage: 'naming', naming: { ...marks.naming, l5ConfirmedOn: null }, levels: marks.vertebrae.map((v) => v.level) },
    { kind: 'stage-started', film, stage: 'confidence' },
    {
      kind: 'confidence',
      film,
      stage: 'confidence',
      levels: marks.vertebrae.map((v) => ({ level: v.level, score: v.confidence.score, band: v.confidence.band })),
    },
    { kind: 'outlines', film, stage: 'confidence', quads, final: true },
  ];
}

/** The frames the contract never drops, made from each film's marks after the pass. */
export function keyframesOnly(marks: FilmMarks[]): WatchFrame[] {
  return [
    ...marks.flatMap((film) => FILM_STAGES.map((stage): WatchFrame => ({ kind: 'stage-started', film: film.film, stage }))),
    ...marks.flatMap(chainFrames),
  ];
}
