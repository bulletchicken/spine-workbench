// Marks as the engine would make them, for the random histories: the
// films of a case to open, and the answers to a pass, a region wand
// click, a split, a merge and a pin, each made from the case as it stood
// when it was asked for, with new ids counting up from the case's
// `nextVertebraId` then. Tests only; they stand in for the engine's
// answers in shape and in ids, not in what they find on a film.

import {
  CONFIDENCE_HIGH_FROM,
  CONFIDENCE_MEDIUM_FROM,
  TEMPLATE_LEVELS,
  TEMPLATE_NAMES,
  type CaseFileV1,
  type Confidence,
  type FilmKind,
  type FilmMarks,
  type HipCentre,
  type ImageRight,
  type Landmarks,
  type Level,
  type Mark,
  type NamingFlag,
  type PedicleMark,
  type PinTarget,
  type Px,
  type VertebraMarks,
  type VertebraOnFilm,
} from '@/shapes';
import type { CaseEdit, MarksCause } from './case-edits';
import { c7Of } from './c7-of';
import { between, pick } from './seeded-numbers';
import { filmOf } from './selectors';
import { FILM_HEIGHT, FILM_WIDTH, isFrontView, usualFacing } from './sketch-case';

const mark = (at: Px, rng: () => number): Mark => ({ at, source: rng() < 0.8 ? 'auto' : 'pin', pinned: rng() < 0.2, inferred: rng() < 0.15 });

function confidence(rng: () => number): Confidence {
  const score = Math.round(rng() * 100) / 100;
  const band = score >= CONFIDENCE_HIGH_FROM ? 'high' : score >= CONFIDENCE_MEDIUM_FROM ? 'medium' : 'low';
  return { score, band, reasons: rng() < 0.3 ? ['weak-gap'] : [], calibrated: false, endplateSpreadPx: null, margin: null };
}

function pedicle(at: Px, rng: () => number): PedicleMark {
  if (rng() < 0.2) return { seen: 'not-visible', source: 'auto' };
  return { seen: 'visible', ...mark(at, rng), ringScore: rng() < 0.5 ? 0.5 : null };
}

function boxAt(film: FilmKind, left: number, top: number, rng: () => number): VertebraMarks {
  const corner = (x: number, y: number) => mark([x, y], rng);
  const [l, r, t, bottom] = [left, left + 70, top, top + 35];
  if (film === 'front') {
    return {
      kind: 'front',
      points: { supL: corner(l, t), supR: corner(r, t), infL: corner(l, bottom), infR: corner(r, bottom), pedL: pedicle([l + 10, t + 10], rng), pedR: pedicle([r - 10, t + 10], rng) },
    };
  }
  if (film === 'side') return { kind: 'side', points: { supA: corner(l, t), supP: corner(r, t), infA: corner(l, bottom), infP: corner(r, bottom) } };
  return { kind: 'bending', lines: { sup: [corner(l, t), corner(r, t)], inf: [corner(l, bottom), corner(r, bottom)] } };
}

function landmarksFor(film: FilmKind, vertebrae: VertebraOnFilm[], rng: () => number): Landmarks {
  const point = (): Mark => mark([between(rng, 0, FILM_WIDTH - 1), between(rng, 1000, FILM_HEIGHT - 1)], rng);
  const sacrumTop: Landmarks['sacrumTop'] = rng() < 0.5 ? null : isFrontView(film) ? { view: 'front', sacL: point(), sacR: point() } : { view: 'side', sacA: point(), sacP: point() };
  const hip = (side: HipCentre['side']): HipCentre => ({ side, centre: point(), radiusPx: rng() < 0.5 ? 24 : null });
  const hips =
    film === 'front'
      ? [hip('left'), hip('right')].filter(() => rng() < 0.6)
      : film === 'side'
        ? (pick([[], [hip('unpaired')], [hip('merged')], [hip('unpaired'), hip('unpaired')]], between(rng, 0, 3)) ?? [])
        : [];
  return { c7: c7Of(vertebrae), sacrumTop, hips };
}

/** One film's marks: `ids.length` vertebrae, a run of the standard count's levels, the film's facing as named. */
export function madeUpMarks(film: FilmKind, ids: readonly number[], namedFor: ImageRight, rng: () => number): FilmMarks {
  const levels = TEMPLATE_LEVELS.standard;
  const first = between(rng, 0, levels.length - ids.length);
  const vertebrae = ids.map((id, index) => ({
    id,
    level: levels[first + index] ?? 'L5',
    marks: boxAt(film, between(rng, 0, 900), 60 + index * 60, rng),
    confidence: confidence(rng),
    outline: null,
  }));
  return {
    film,
    vertebrae,
    landmarks: landmarksFor(film, vertebrae, rng),
    naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: null },
    namedFor,
  };
}

const namedForNow = (c: CaseFileV1, film: FilmKind): ImageRight => filmOf(c, film)?.geometry.imageRight ?? usualFacing(film);

export function answerOf(marks: FilmMarks, basedOn: number, cause: MarksCause): CaseEdit {
  return { kind: 'replace-marks', marks, basedOn, cause };
}

/** The pass's answers: every film, in order, new ids counting up from the case's next id across the films. */
export function passAnswers(c: CaseFileV1, version: number, rng: () => number): CaseEdit[] {
  let next = c.nextVertebraId;
  return c.films.map((film) => {
    const ids = Array.from({ length: between(rng, 0, 4) }, (_, index) => next + index);
    next += ids.length;
    return answerOf(madeUpMarks(film.kind, ids, namedForNow(c, film.kind), rng), version, { by: 'auto-pass' });
  });
}

export function withVertebra(marks: FilmMarks, index: number, vertebra: VertebraOnFilm): FilmMarks {
  const vertebrae = marks.vertebrae.with(index, vertebra);
  return { ...marks, vertebrae, landmarks: { ...marks.landmarks, c7: c7Of(vertebrae) } };
}

/** The vertebra with every corner moved by the same few pixels, refitted by the wand. */
export function nudged(vertebra: VertebraOnFilm, rng: () => number): VertebraOnFilm {
  const [dx, dy] = [between(rng, -5, 5), between(rng, -5, 5)];
  const move = (m: Mark): Mark => ({ ...m, at: [m.at[0] + dx, m.at[1] + dy], source: 'region-wand' });
  const marks = vertebra.marks;
  const moved: VertebraMarks =
    marks.kind === 'front'
      ? { kind: 'front', points: { ...marks.points, supL: move(marks.points.supL), supR: move(marks.points.supR), infL: move(marks.points.infL), infR: move(marks.points.infR) } }
      : marks.kind === 'side'
        ? { kind: 'side', points: { supA: move(marks.points.supA), supP: move(marks.points.supP), infA: move(marks.points.infA), infP: move(marks.points.infP) } }
        : { kind: 'bending', lines: { sup: [move(marks.lines.sup[0]), move(marks.lines.sup[1])], inf: [move(marks.lines.inf[0]), move(marks.lines.inf[1])] } };
  return { ...vertebra, marks: moved, confidence: { ...vertebra.confidence, reasons: ['edited-by-user'] } };
}

/**
 * The film's marks with these vertebrae after a split or merge: flagged
 * when a level falls outside the count, the two counts to be made again,
 * and a confirmed L5 kept only while that vertebra is still L5.
 */
function renamed(marks: FilmMarks, vertebrae: VertebraOnFilm[]): FilmMarks {
  const template = marks.naming.template;
  const outside = vertebrae.some((vertebra) => !TEMPLATE_LEVELS[template].includes(vertebra.level));
  const flags: NamingFlag[] = outside && !marks.naming.flags.includes('non-standard-count') ? [...marks.naming.flags, 'non-standard-count'] : marks.naming.flags;
  const confirmed = marks.naming.l5ConfirmedOn;
  const stillL5 = vertebrae.some((vertebra) => vertebra.id === confirmed && vertebra.level === 'L5');
  return {
    ...marks,
    vertebrae,
    landmarks: { ...marks.landmarks, c7: c7Of(vertebrae) },
    naming: { ...marks.naming, fromTop: null, fromBottom: null, flags, l5ConfirmedOn: stillL5 ? confirmed : null },
  };
}

function shiftedLevel(marks: FilmMarks, level: Level, by: number): Level | null {
  const names = TEMPLATE_NAMES[marks.naming.template];
  return names[names.indexOf(level) + by] ?? null;
}

/**
 * The vertebra at `index` split in two, the lower piece taking `newId`,
 * and the names below it one step lower; `null` when a name would run
 * past the end of the count's names, which the engine refuses.
 */
export function splitMarks(marks: FilmMarks, index: number, newId: number): FilmMarks | null {
  const upper = marks.vertebrae[index];
  if (upper === undefined) return null;
  const below: VertebraOnFilm[] = [];
  for (const vertebra of [{ ...upper, id: newId }, ...marks.vertebrae.slice(index + 1)]) {
    const level = shiftedLevel(marks, vertebra.level, 1);
    if (level === null) return null;
    below.push({ ...vertebra, level });
  }
  return renamed(marks, [...marks.vertebrae.slice(0, index + 1), ...below]);
}

/**
 * The vertebrae at `index` and `index + 1` merged into the upper one, the
 * names below one step higher; `null` when there is no pair there.
 */
export function mergedMarks(marks: FilmMarks, index: number): FilmMarks | null {
  if (index < 0 || index + 1 >= marks.vertebrae.length) return null;
  const below: VertebraOnFilm[] = [];
  for (const vertebra of marks.vertebrae.slice(index + 2)) {
    const level = shiftedLevel(marks, vertebra.level, -1);
    if (level === null) return null;
    below.push({ ...vertebra, level });
  }
  return renamed(marks, [...marks.vertebrae.slice(0, index + 1), ...below]);
}

/** The engine's answer to a pin: that point exactly where it was dragged, pinned, the rest as it was. */
export function pinned(marks: FilmMarks, target: PinTarget, at: Px): FilmMarks {
  if (target.on === 'landmark') return marks;
  const index = marks.vertebrae.findIndex((vertebra) => vertebra.id === target.vertebra);
  const vertebra = marks.vertebrae[index];
  if (vertebra === undefined) return marks;
  const placed: Mark = { at, source: 'pin', pinned: true, inferred: false };
  const m = vertebra.marks;
  let moved: VertebraMarks = m;
  if (target.on === 'endplate' && m.kind === 'bending') {
    const line = m.lines[target.line];
    moved = { kind: 'bending', lines: { ...m.lines, [target.line]: target.end === 'patient-left' ? [placed, line[1]] : [line[0], placed] } };
  } else if (target.on === 'vertebra' && m.kind === 'front' && (target.mark === 'supL' || target.mark === 'supR' || target.mark === 'infL' || target.mark === 'infR')) {
    moved = { kind: 'front', points: { ...m.points, [target.mark]: placed } };
  } else if (target.on === 'vertebra' && m.kind === 'side' && (target.mark === 'supA' || target.mark === 'supP' || target.mark === 'infA' || target.mark === 'infP')) {
    moved = { kind: 'side', points: { ...m.points, [target.mark]: placed } };
  }
  return withVertebra(marks, index, { ...vertebra, marks: moved });
}
