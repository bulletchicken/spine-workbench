// Small cases drawn by hand for the case-state tests: a film record, and
// a film's marks as a row of upright boxes, one per level, every value
// one the strict reader accepts. Tests only; nothing in the app imports
// this file.

import {
  TEMPLATE_LEVELS,
  type CaseFileV1,
  type Film,
  type FilmKind,
  type FilmMarks,
  type ImageRight,
  type Level,
  type Mark,
  type VertebraMarks,
  type VertebraOnFilm,
} from '@/shapes';
import { c7Of } from './c7-of';
import { emptyCase } from './empty-case';

export const FILM_WIDTH = 1000;
export const FILM_HEIGHT = 2000;

/** Each box is this tall, with a gap of `GAP` below it. */
export const BOX = 40;
export const GAP = 10;

/** Whether a film kind is a front view (front and bending films) or a side view. */
export const isFrontView = (kind: FilmKind): boolean => kind !== 'side';

/** The facing a film's names are given for when nothing says otherwise. */
export const usualFacing = (kind: FilmKind): ImageRight => (isFrontView(kind) ? 'patient-left' : 'anterior');

/** A PNG film's record, upright, nothing known or confirmed yet. */
export function sketchFilm(kind: FilmKind): Film {
  return {
    kind,
    image: { path: `films/${kind}.png`, width: FILM_WIDTH, height: FILM_HEIGHT, bitsStored: 8 },
    source: 'png',
    keptTags: null,
    values: { offset: 0, inverted: false, invertedByUser: false, modality: { kind: 'linear', slope: 1, intercept: 0 } },
    flags: [],
    blanked: [],
    geometry: {
      mmPerPx: null,
      mmPerPxVertical: null,
      scale: null,
      quarterTurns: 0,
      imageRight: null,
      confirmed: { upright: false, leftRight: false, facing: false, scale: false, polarity: false, privacy: false },
    },
  };
}

export const auto = (x: number, y: number): Mark => ({ at: [x, y], source: 'auto', pinned: false, inferred: false });

/**
 * The marks of one box with its top-left corner at (`left`, `top`): image
 * left is patient left (or anterior), as for the usual facing.
 */
export function boxMarks(kind: FilmKind, left: number, top: number): VertebraMarks {
  const [l, r, t, b] = [left, left + BOX * 2, top, top + BOX];
  if (kind === 'front') {
    return {
      kind: 'front',
      points: {
        supL: auto(l, t),
        supR: auto(r, t),
        infL: auto(l, b),
        infR: auto(r, b),
        pedL: { seen: 'visible', at: [l + 10, t + 12], source: 'auto', pinned: false, inferred: false, ringScore: 0.5 },
        pedR: { seen: 'visible', at: [r - 10, t + 12], source: 'auto', pinned: false, inferred: false, ringScore: 0.5 },
      },
    };
  }
  if (kind === 'side') return { kind: 'side', points: { supA: auto(l, t), supP: auto(r, t), infA: auto(l, b), infP: auto(r, b) } };
  return { kind: 'bending', lines: { sup: [auto(l, t), auto(r, t)], inf: [auto(l, b), auto(r, b)] } };
}

/** A vertebra with high confidence and no outline. */
export function sketchVertebra(kind: FilmKind, id: number, level: Level, left: number, top: number): VertebraOnFilm {
  return {
    id,
    level,
    marks: boxMarks(kind, left, top),
    confidence: { score: 0.95, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
    outline: null,
  };
}

/**
 * One film's marks: a box per level, top to bottom, ids counting up from
 * `firstId`, named for the usual facing, with its C7 landmark worked out.
 */
export function sketchMarks(kind: FilmKind, firstId: number, levels: readonly Level[]): FilmMarks {
  const vertebrae = levels.map((level, index) => sketchVertebra(kind, firstId + index, level, 100, 100 + index * (BOX + GAP)));
  return {
    film: kind,
    vertebrae,
    landmarks: { c7: c7Of(vertebrae), sacrumTop: null, hips: [] },
    naming: {
      template: 'standard',
      chosenBy: 'engine',
      alternatives: [],
      fromTop: null,
      fromBottom: null,
      flags: [],
      l5ConfirmedOn: null,
    },
    namedFor: usualFacing(kind),
  };
}

/** The standard count's levels from `from`, `count` of them. */
export function standardLevels(from: Level, count: number): Level[] {
  const levels = TEMPLATE_LEVELS.standard;
  const start = levels.indexOf(from);
  return levels.slice(start, start + count);
}

/** A case holding these films, and these marks, with ids given out up to the highest. */
export function sketchCase(films: readonly FilmKind[], marks: readonly FilmMarks[] = []): CaseFileV1 {
  const ids = marks.flatMap((filmMarks) => filmMarks.vertebrae.map((vertebra) => vertebra.id));
  return { ...emptyCase(), films: films.map(sketchFilm), marks: [...marks], nextVertebraId: Math.max(1, ...ids.map((id) => id + 1)) };
}
