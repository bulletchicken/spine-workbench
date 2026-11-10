// Mark names as lists, in their fixed order, and the order of levels.
//
// Each mark list is built from a record keyed by the generated union, so
// the compiler checks both ways: a name missing from the record, or one
// the union does not have, is an error. Object keys keep the order they
// are written in, which is the order below. The levels in order, and each
// vertebra count's levels and names, are generated from the Rust shapes
// into generated/constants.ts.

import { LEVELS } from './generated/constants';
import type { FrontMarks } from './generated/FrontMarks';
import type { Level } from './generated/Level';
import type { MarkName } from './generated/MarkName';
import type { SideMarks } from './generated/SideMarks';

const FRONT: Record<keyof FrontMarks, true> = {
  supL: true,
  supR: true,
  infL: true,
  infR: true,
  pedL: true,
  pedR: true,
};

const SIDE: Record<keyof SideMarks, true> = {
  supA: true,
  supP: true,
  infA: true,
  infP: true,
};

/**
 * Which film kind each vertebra mark belongs to. The annotation checks
 * that the front and side names together are exactly `MarkName`. Bending
 * films have no mark names: their points are an endplate line and an end
 * (`PinTarget` `endplate`).
 */
export const MARK_FILM: { readonly [N in keyof FrontMarks]: 'front' } & {
  readonly [N in keyof SideMarks]: 'side';
} = {
  supL: 'front',
  supR: 'front',
  infL: 'front',
  infR: 'front',
  pedL: 'front',
  pedR: 'front',
  supA: 'side',
  supP: 'side',
  infA: 'side',
  infP: 'side',
} satisfies Record<MarkName, 'front' | 'side'>;

/** The front-film marks: corners patient left and right, then the pedicles. */
export const FRONT_MARKS = Object.keys(FRONT) as readonly (keyof FrontMarks)[];

/** The side-film corners: anterior and posterior. */
export const SIDE_MARKS = Object.keys(SIDE) as readonly (keyof SideMarks)[];

/** Negative when `a` is above `b`, for sorting top to bottom. */
export function compareLevels(a: Level, b: Level): number {
  return LEVELS.indexOf(a) - LEVELS.indexOf(b);
}
