// Each sample film's marks as the automatic pass leaves them: the design's
// story, on top of the chain the engine always solves.
//
// Front and side carry the 17 outlines the design counts, plus the C7 body
// above them that the engine's standard chain always holds, so 18 bodies:
// - T11 and T12 taken as one outline, twice as tall as its neighbours, in
//   the low band with the reason `odd-height`;
// - the lowest outline sitting on the top of the sacrum, in the medium
//   band with the reason `level-uncertain`;
// - T3 in the low band on the front film, its outline short of the bone on
//   the patient's left (the shoulder's shadow), `rib-overlap`;
// - T1 in the medium band, with its left pedicle not visible on the front
//   film (`pedicle-hidden`) and a faint gap on the side film (`weak-gap`).
// Counted from the first rib the outline on the sacrum is L5; counted up
// from the sacrum the one above it is. The two counts are one apart, and
// the pass names the chain from the top.
//
// The bending films' chains are found right: C7 to L5, 18 bodies, each two
// endplate lines, both counts agreeing. A bending film's count cannot be
// changed by any edit, so a merged outline there could never be split and
// the story could never end with everything checked.
//
// Ids count from 0, film after film in the case's order; the stand-in
// adds the pass's first vertebra id, as the engine does.

import type {
  Confidence,
  ConfidenceBand,
  ConfidenceReason,
  FilmKind,
  FilmMarks,
  Landmarks,
  Level,
  LevelNaming,
  Mark,
  PedicleMark,
  Px,
  VertebraMarks,
  VertebraOnFilm,
} from '@/shapes';
import { TEMPLATE_LEVELS } from '@/shapes';
import { boneOf, FILM_KINDS, FRONT_HIPS, sideHips, toFilmPixels, type Point } from './anatomy';

/** Points to a hundredth of a pixel, so the fixture reads cleanly. */
const tidy = (value: number) => Math.round(value * 100) / 100;

export function filmPoint(kind: FilmKind, point: Point): Px {
  const [x, y] = toFilmPixels(kind, point);
  return [tidy(x), tidy(y)];
}

export const autoMark = (at: Px): Mark => ({ at, source: 'auto', pinned: false, inferred: false });

/** Which bones an outline covers, top and bottom; one bone unless the pass merged two. */
export interface Outline {
  top: number;
  bottom: number;
}

const single = (bone: number): Outline => ({ top: bone, bottom: bone });

/** The front and side chain: C7, T1 to T10, T11 and T12 as one, L1 to L5, and S1. */
export const FRONT_AND_SIDE_CHAIN: readonly Outline[] = [
  ...Array.from({ length: 11 }, (_, bone) => single(bone)),
  { top: 11, bottom: 12 },
  ...Array.from({ length: 6 }, (_, k) => single(13 + k)),
];

/** The bending chain: C7 to L5, each its own. */
export const BENDING_CHAIN: readonly Outline[] = Array.from({ length: 18 }, (_, bone) => single(bone));

/** The levels the pass gives a chain of 18 from the top: C7 to L5. */
export const STANDARD_LEVELS: readonly Level[] = TEMPLATE_LEVELS.standard;

/** The front and side chain counted up from the sacrum: every name one further down. */
export const FROM_THE_SACRUM: readonly Level[] = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'];

export function confidenceOf(
  score: number,
  band: ConfidenceBand,
  reasons: ConfidenceReason[],
  spread: [number, number],
  margin: number,
): Confidence {
  return { score, band, reasons, calibrated: false, endplateSpreadPx: { sup: spread[0], inf: spread[1] }, margin };
}

const sure = (score: number, sup = 1.2, inf = 1.4) => confidenceOf(score, 'high', [], [sup, inf], 6.5);

/** Each outline's confidence on each film, top to bottom. */
const CONFIDENCE: Record<FilmKind, Confidence[]> = {
  front: [
    sure(0.93, 1.4, 1.6),
    confidenceOf(0.74, 'medium', ['pedicle-hidden'], [2.1, 2.4], 3.1),
    sure(0.95),
    confidenceOf(0.38, 'low', ['rib-overlap'], [4.6, 7.9], 0.6),
    sure(0.94),
    sure(0.96),
    sure(0.97, 1.1, 1.2),
    sure(0.97, 1.1, 1.2),
    sure(0.96),
    sure(0.95),
    sure(0.94),
    confidenceOf(0.41, 'low', ['odd-height'], [3.2, 3.6], 0.4),
    sure(0.93, 1.5, 1.4),
    sure(0.96),
    sure(0.97, 1.0, 1.1),
    sure(0.96),
    sure(0.95, 1.3, 1.5),
    confidenceOf(0.67, 'medium', ['level-uncertain'], [2.6, 2.9], 1.8),
  ],
  side: [
    sure(0.91, 1.6, 1.8),
    confidenceOf(0.71, 'medium', ['weak-gap'], [2.4, 2.2], 2.7),
    sure(0.94),
    sure(0.92, 1.5, 1.6),
    sure(0.95),
    sure(0.96),
    sure(0.96),
    sure(0.97, 1.1, 1.2),
    sure(0.96),
    sure(0.95),
    sure(0.93),
    confidenceOf(0.44, 'low', ['odd-height'], [3.0, 3.4], 0.5),
    sure(0.94, 1.4, 1.3),
    sure(0.96),
    sure(0.97, 1.0, 1.1),
    sure(0.96),
    sure(0.94, 1.3, 1.5),
    confidenceOf(0.69, 'medium', ['level-uncertain'], [2.5, 2.8], 1.9),
  ],
  'bend-left': BENDING_CHAIN.map((_, k) => sure([0.91, 0.93, 0.94][k % 3] ?? 0.93, 1.5, 1.7)),
  'bend-right': BENDING_CHAIN.map((_, k) => sure([0.92, 0.94, 0.93][k % 3] ?? 0.93, 1.5, 1.7)),
};

/**
 * An outline's four corners on the upright picture, image left to right:
 * top left, top right, bottom right, bottom left. On the front film T3
 * stops short of its bone on the image right, as in the mock.
 */
function cornersOf(kind: FilmKind, outline: Outline): [Point, Point, Point, Point] {
  const top = boneOf(kind, outline.top);
  const bottom = boneOf(kind, outline.bottom);
  const q: [Point, Point, Point, Point] = [{ ...top.q[0] }, { ...top.q[1] }, { ...bottom.q[2] }, { ...bottom.q[3] }];
  if (kind === 'front' && outline.top === 3) {
    q[1] = { x: q[1].x - top.p.x * 17, y: q[1].y - top.p.y * 17 };
    q[2] = { x: q[2].x - bottom.p.x * 20, y: q[2].y - bottom.p.y * 20 };
  }
  return q;
}

/**
 * Where an inferred pedicle goes in a quad with no ring to find:
 * (u, v) = (0.11, 0.32) for the left one and (0.89, 0.32) for the right,
 * across from the left corners and down from the upper endplate.
 */
export function defaultPedicles(supL: Px, supR: Px, infL: Px, infR: Px): { pedL: Px; pedR: Px } {
  const at = (u: number, v: number): Px => {
    const top = [supL[0] + (supR[0] - supL[0]) * u, supL[1] + (supR[1] - supL[1]) * u];
    const bottom = [infL[0] + (infR[0] - infL[0]) * u, infL[1] + (infR[1] - infL[1]) * u];
    return [tidy((top[0] ?? 0) + ((bottom[0] ?? 0) - (top[0] ?? 0)) * v), tidy((top[1] ?? 0) + ((bottom[1] ?? 0) - (top[1] ?? 0)) * v)];
  };
  return { pedL: at(0.11, 0.32), pedR: at(0.89, 0.32) };
}

export const inferredPedicle = (at: Px, source: Mark['source']): PedicleMark => ({
  seen: 'visible',
  at,
  source,
  pinned: false,
  inferred: true,
  ringScore: null,
});

function marksOf(kind: FilmKind, outline: Outline): VertebraMarks {
  const [tl, tr, br, bl] = cornersOf(kind, outline).map((point) => filmPoint(kind, point)) as [Px, Px, Px, Px];
  if (kind === 'side') {
    // The side film faces right: anterior is on the image right.
    return { kind: 'side', points: { supA: autoMark(tr), supP: autoMark(tl), infA: autoMark(br), infP: autoMark(bl) } };
  }
  // Front-view films have the patient's left on the image right.
  if (kind !== 'front') return { kind: 'bending', lines: { sup: [autoMark(tr), autoMark(tl)], inf: [autoMark(br), autoMark(bl)] } };
  const truth = boneOf(kind, outline.top).ped;
  let pedL: PedicleMark;
  let pedR: PedicleMark;
  if (truth === null) {
    const fallback = defaultPedicles(tr, tl, br, bl);
    pedL = inferredPedicle(fallback.pedL, 'auto');
    pedR = inferredPedicle(fallback.pedR, 'auto');
  } else {
    const ring = 0.62 + 0.03 * (outline.top % 9);
    const placed = (pedicle: (typeof truth)[number]): PedicleMark =>
      pedicle.seen
        ? { seen: 'visible', at: filmPoint(kind, pedicle), source: 'auto', pinned: false, inferred: false, ringScore: tidy(ring) }
        : { seen: 'not-visible', source: 'auto' };
    pedL = placed(truth[1]);
    pedR = placed(truth[0]);
  }
  return {
    kind: 'front',
    points: { supL: autoMark(tr), supR: autoMark(tl), infL: autoMark(br), infR: autoMark(bl), pedL, pedR },
  };
}

/** A vertebra's four corners (a bending film's line ends) in quad order. */
export function quadOf(marks: VertebraMarks): [Mark, Mark, Mark, Mark] {
  switch (marks.kind) {
    case 'front':
      return [marks.points.supL, marks.points.supR, marks.points.infR, marks.points.infL];
    case 'side':
      return [marks.points.supA, marks.points.supP, marks.points.infP, marks.points.infA];
    case 'bending':
      return [marks.lines.sup[0], marks.lines.sup[1], marks.lines.inf[1], marks.lines.inf[0]];
  }
}

/** `landmarks.c7` as `Landmarks::c7_of` works it out: the mean of the C7 vertebra's corners, added in quad order. */
export function c7Of(vertebrae: readonly VertebraOnFilm[]): Mark | null {
  const c7 = vertebrae.find((vertebra) => vertebra.level === 'C7');
  if (c7 === undefined) return null;
  const corners = quadOf(c7.marks);
  const mean = (axis: 0 | 1) => corners.reduce((sum, mark) => sum + mark.at[axis], 0) / 4;
  return { at: [mean(0), mean(1)], source: 'auto', pinned: false, inferred: corners.some((mark) => mark.inferred) };
}

function landmarksOf(kind: FilmKind, vertebrae: VertebraOnFilm[]): Landmarks {
  const sacrum = boneOf(kind, 18);
  const left = autoMark(filmPoint(kind, sacrum.q[1]));
  const right = autoMark(filmPoint(kind, sacrum.q[0]));
  if (kind === 'side') {
    const [first, second] = sideHips();
    return {
      c7: c7Of(vertebrae),
      sacrumTop: { view: 'side', sacA: left, sacP: right },
      hips: [
        { side: 'unpaired', centre: autoMark(filmPoint(kind, first)), radiusPx: first.r },
        { side: 'unpaired', centre: autoMark(filmPoint(kind, second)), radiusPx: second.r },
      ],
    };
  }
  const [imageLeft, imageRight] = FRONT_HIPS;
  return {
    c7: c7Of(vertebrae),
    sacrumTop: { view: 'front', sacL: left, sacR: right },
    hips:
      kind === 'front'
        ? [
            { side: 'left', centre: autoMark(filmPoint(kind, imageRight)), radiusPx: imageRight.r },
            { side: 'right', centre: autoMark(filmPoint(kind, imageLeft)), radiusPx: imageLeft.r },
          ]
        : [],
  };
}

function namingOf(kind: FilmKind): LevelNaming {
  if (kind === 'front' || kind === 'side') {
    return {
      template: 'standard',
      chosenBy: 'engine',
      alternatives: [
        { template: 'standard', chance: 0.71 },
        { template: 'extra-lumbar', chance: 0.17 },
        { template: 'missing-thoracic', chance: 0.12 },
      ],
      fromTop: [...STANDARD_LEVELS],
      fromBottom: [...FROM_THE_SACRUM],
      flags: ['ends-disagree'],
      l5ConfirmedOn: null,
    };
  }
  return {
    template: 'standard',
    chosenBy: 'engine',
    alternatives: [
      { template: 'standard', chance: 0.9 },
      { template: 'extra-lumbar', chance: 0.06 },
      { template: 'missing-lumbar', chance: 0.04 },
    ],
    fromTop: [...STANDARD_LEVELS],
    fromBottom: [...STANDARD_LEVELS],
    flags: [],
    l5ConfirmedOn: null,
  };
}

/** One film's marks as the pass leaves them, its ids counting from `firstId`. */
function filmMarks(kind: FilmKind, firstId: number): FilmMarks {
  const chain = kind === 'front' || kind === 'side' ? FRONT_AND_SIDE_CHAIN : BENDING_CHAIN;
  const vertebrae = chain.map(
    (outline, k): VertebraOnFilm => ({
      id: firstId + k,
      level: STANDARD_LEVELS[k] ?? 'L5',
      marks: marksOf(kind, outline),
      confidence: structuredClone(CONFIDENCE[kind][k] ?? sure(0.9)),
      outline: null,
    }),
  );
  return {
    film: kind,
    vertebrae,
    landmarks: landmarksOf(kind, vertebrae),
    naming: namingOf(kind),
    namedFor: kind === 'side' ? 'anterior' : 'patient-left',
  };
}

/** Every film's marks after the pass, ids from 0, in the case's order of films. */
export function passMarks(): FilmMarks[] {
  let next = 0;
  return FILM_KINDS.map((kind) => {
    const marks = filmMarks(kind, next);
    next += marks.vertebrae.length;
    return marks;
  });
}

/** The same marks with every id raised by `first`, as the stand-in hands them back. */
export function withIdsFrom(marks: FilmMarks[], first: number): FilmMarks[] {
  return marks.map((film) => ({
    ...film,
    vertebrae: film.vertebrae.map((vertebra) => ({ ...vertebra, id: vertebra.id + first })),
    naming: { ...film.naming, l5ConfirmedOn: film.naming.l5ConfirmedOn === null ? null : film.naming.l5ConfirmedOn + first },
  }));
}
