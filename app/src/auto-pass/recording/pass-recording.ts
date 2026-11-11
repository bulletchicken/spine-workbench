// The PassRecording: everything the automatic pass found, per film, kept
// as the frames carried it and turned into what the screens draw. The
// pass is played from this at its own pace and never recomputed.
//
// Every value here comes from a watch frame or from the pass's answer.
// Bitmaps are the frames' bytes laid out for drawing, and a bitmap's place
// on the film is its tile's origin and steps, so nothing is made up.

import type { Matrix } from '@/film-viewer/film-space';
import type {
  AutoPassResult,
  ChainBody,
  FilmKind,
  Level,
  LevelConfidence,
  LevelNaming,
  OutlineQuad,
  PassWarning,
  Px,
  PxRect,
  Stage,
  WatchFrame,
  WidthSample,
} from '@/shapes';

/** The three finished evidence maps the pass shows; a film without pedicle rings has two. */
export type ShownMap = 'phase-edge' | 'ridge' | 'ring';
export const SHOWN_MAPS: readonly ShownMap[] = ['phase-edge', 'ridge', 'ring'];

/**
 * A bitmap and where it sits on the film. Pixel (x, y) of the bitmap
 * covers the square from (x, y) to (x + 1, y + 1); `toFilm` takes those
 * coordinates to film pixels, in the order of SVG's matrix().
 */
export interface TileImage {
  width: number;
  height: number;
  /** Row-major RGBA, ready for an ImageData. */
  rgba: Uint8ClampedArray;
  toFilm: Matrix;
}

export interface EndZones {
  top: PxRect;
  bottom: PxRect;
  topCropped: boolean;
  bottomCropped: boolean;
}

/** Where each strip row sat on the film: row r is the line through `centres[r]` along `normals[r]`. */
export interface StripShape {
  centres: Px[];
  /** Unit vectors in film pixels, pointing to strip column +1. */
  normals: Px[];
  /** Film pixels per strip column. */
  columnPx: number;
  columns: number;
}

/** The unrolled strip, head at row 0. Cells that were not usable are fully transparent. */
export interface StripImage {
  rows: number;
  columns: number;
  /** Row-major RGBA, ready for an ImageData. */
  rgba: Uint8ClampedArray;
}

/** The bone and gap profile down the strip, one value per strip row; NaN where a row was not usable. */
export interface Barcode {
  values: Float32Array;
  /** The same with slow trends removed: bone above zero, gaps below. */
  highPass: Float32Array;
}

/** A film's strip, whole: what the ladder and the gap finder need. */
export interface FilmStrip {
  shape: StripShape;
  image: StripImage;
  barcode: Barcode;
}

/** One block of the strip as it arrived, kept until every row has come. */
export type StripBlock = Extract<WatchFrame, { kind: 'strip-rows' }>;

/** One step of the search down the chain. */
export interface SweepRecord {
  /** The level the search has reached. */
  level: Level;
  /** 256 bins from the top of the strip to the bottom; 255 is best. */
  heat: Uint8Array;
  /** The best chain so far ending at this level. */
  tentative: ChainBody[];
}

export interface Naming {
  naming: LevelNaming;
  /** The level of each locked body, top to bottom. */
  levels: Level[];
}

export interface FilmRecording {
  film: FilmKind;
  /** The stages whose `stage-started` frame has arrived. */
  stagesStarted: Stage[];
  /** The finished evidence maps the pass shows, by map. */
  maps: Partial<Record<ShownMap, TileImage>>;
  endZones: EndZones | null;
  /** How far the centre line's search had spread, in the order of their `step`. */
  wavefront: { step: number; image: TileImage }[];
  coarseLine: Px[] | null;
  refinedLine: Px[] | null;
  width: WidthSample[] | null;
  stripShape: StripShape | null;
  /** Blocks of the strip waiting for the rest; empty once `strip` is whole. */
  stripBlocks: StripBlock[];
  strip: StripImage | null;
  barcode: Barcode | null;
  /** Every sweep frame, in the order it arrived. */
  sweeps: SweepRecord[];
  /** The locked chain, from the sacrum upward: the order the bodies lock in. */
  locked: ChainBody[] | null;
  named: Naming | null;
  confidence: LevelConfidence[] | null;
  /** The final outlines. */
  outlines: OutlineQuad[] | null;
  warnings: PassWarning[];
}

export interface PassRecording {
  /** One part per film the pass runs, in the order of the pass's input. */
  films: FilmRecording[];
  /** The pass's answer, once it has returned. */
  result: AutoPassResult | null;
}
