// The four sample films, each as the file a person would drop, what
// reading it finds out, its pixels for the GPU and its thumbnail.
//
// Front, side and bending left are DICOM files whose pixel spacing gives
// their size; bending right is a PNG, which has none, so it is the film
// whose size is measured with the two-click ruler. The bending-left film
// is stored upside down, as if the cassette went in the wrong way up.
//
// Everything is made the first time it is asked for, because painting a
// film takes a moment, and each answer is a fresh copy, so a caller that
// hands a buffer to a worker cannot empty it for the next one.

import type { DisplayWindow, FilmFacts, FilmKind, KeptTags, PixelValues } from '@/shapes';
import { FILM_HEIGHT, FILM_WIDTH, MM_PER_PX } from './anatomy';
import { paintedFilm } from './film-painter';
import { DX_FOR_PRESENTATION, writeDicom } from './files/dicom-file';
import { halfFloatBits } from './files/half-float';
import { writePng } from './files/png-file';

/** The films whose files carry their size. */
export const SIZED_BY_FILE: readonly FilmKind[] = ['front', 'side', 'bend-left'];

/** A film's thumbnail fits in this many pixels on its longest side. */
export const THUMBNAIL_SIDE = 256;

export interface SampleFilm {
  kind: FilmKind;
  /** The bytes a person drops: a DICOM file, or for bending right a PNG. */
  readonly file: ArrayBuffer;
  /** What reading the file finds out. */
  readonly facts: FilmFacts;
  /** An 8-bit greyscale PNG, at most 256 pixels on its longest side. */
  readonly thumbnail: Uint8Array;
  /** Half-float bits of the stored pixels, 0 to 1, `facts.width` × `facts.height`. */
  pixels(): Uint16Array;
}

const AS_STORED: PixelValues = {
  offset: 0,
  inverted: false,
  invertedByUser: false,
  modality: { kind: 'linear', slope: 1, intercept: 0 },
};

function memo<T>(make: () => T): () => T {
  let made: { value: T } | undefined;
  return () => {
    made ??= { value: make() };
    return made.value;
  };
}

const storedGreys = new Map<FilmKind, Uint8Array>();

/** A film's pixels as stored: the painted film, turned over for bending left. */
export function storedGrey(kind: FilmKind): Uint8Array {
  let grey = storedGreys.get(kind);
  if (grey === undefined) {
    const upright = paintedFilm(kind).film;
    grey = kind === 'bend-left' ? upright.slice().reverse() : upright;
    storedGreys.set(kind, grey);
  }
  return grey;
}

/** The 1st and 99th percentiles of a film's values: the window a film without one opens with. */
export function percentileWindow(grey: Uint8Array): { centre: number; width: number } {
  const counts = new Array<number>(256).fill(0);
  for (const value of grey) counts[value] = (counts[value] ?? 0) + 1;
  const valueAt = (fraction: number) => {
    const wanted = fraction * grey.length;
    let seen = 0;
    for (let value = 0; value < 256; value++) {
      seen += counts[value] ?? 0;
      if (seen >= wanted) return value;
    }
    return 255;
  };
  const low = valueAt(0.01);
  const high = valueAt(0.99);
  return { centre: (low + high) / 2, width: Math.max(1, high - low) };
}

/** The text a sample PNG carries in its tEXt chunk, naming its film. */
export const pngTitle = (kind: FilmKind) => ({ keyword: 'Title', value: `Spine Workbench sample case: ${kind} film` });

const pngs = new Map<FilmKind, Uint8Array>();

/** A film as an 8-bit greyscale PNG with its title, as a made-up spine hands its films over; a fresh copy. */
export function pngOf(kind: FilmKind): Uint8Array<ArrayBuffer> {
  let png = pngs.get(kind);
  if (png === undefined) {
    png = writePng(storedGrey(kind), FILM_WIDTH, FILM_HEIGHT, pngTitle(kind));
    pngs.set(kind, png);
  }
  return png.slice();
}

function keptTags(kind: FilmKind, windowing: { centre: number; width: number }): KeptTags {
  return {
    sopClass: DX_FOR_PRESENTATION,
    modality: 'DX',
    presentationIntent: 'for-presentation',
    manufacturer: null,
    modelName: null,
    photometric: 'monochrome2',
    bitsStored: 8,
    pixelSpacing: [MM_PER_PX, MM_PER_PX],
    imagerPixelSpacing: null,
    nominalScannedPixelSpacing: null,
    pixelSpacingCalibration: 'geometry',
    estimatedMagnification: null,
    distanceSourceToDetector: null,
    distanceSourceToPatient: null,
    pixelAspectRatio: null,
    patientOrientation: null,
    viewPosition: kind === 'side' ? 'll' : 'ap',
    imageLaterality: null,
    rescaleSlope: null,
    rescaleIntercept: null,
    windowCentre: windowing.centre,
    windowWidth: windowing.width,
    windowFunction: null,
    shutters: null,
    lossyCompression: null,
    lossyRatio: null,
    burnedInAnnotation: null,
    transferSyntax: 'explicit-little-endian',
    numberOfFrames: null,
  };
}

/** What reading a PNG of this film finds: no tags, no size, the percentile window. */
export function pngFacts(kind: FilmKind): FilmFacts {
  const windowing = percentileWindow(storedGrey(kind));
  return {
    kind,
    width: FILM_WIDTH,
    height: FILM_HEIGHT,
    bitsStored: 8,
    source: 'png',
    keptTags: null,
    values: AS_STORED,
    flags: [],
    defaultWindow: { kind: 'centre-width', ...windowing, function: 'linear', from: 'percentile' },
    exifQuarterTurns: null,
  };
}

function dicomFacts(kind: FilmKind): FilmFacts {
  const windowing = percentileWindow(storedGrey(kind));
  const defaultWindow: DisplayWindow = { kind: 'centre-width', ...windowing, function: 'linear', from: 'file' };
  return {
    kind,
    width: FILM_WIDTH,
    height: FILM_HEIGHT,
    bitsStored: 8,
    source: 'dicom',
    keptTags: keptTags(kind, windowing),
    values: AS_STORED,
    flags: [],
    defaultWindow,
    exifQuarterTurns: null,
  };
}

/** The thumbnail's pixels: each the mean of the film pixels whose centres fall inside it. A film already small enough is kept at its size. */
export function thumbnailGrey(grey: Uint8Array, width: number, height: number): { grey: Uint8Array; width: number; height: number } {
  const scale = Math.min(1, THUMBNAIL_SIDE / Math.max(width, height));
  const tw = Math.max(1, Math.round(width * scale));
  const th = Math.max(1, Math.round(height * scale));
  const sums = new Float64Array(tw * th);
  const counts = new Uint32Array(tw * th);
  for (let y = 0; y < height; y++) {
    const ty = Math.min(th - 1, Math.floor(((y + 0.5) * th) / height));
    for (let x = 0; x < width; x++) {
      const tx = Math.min(tw - 1, Math.floor(((x + 0.5) * tw) / width));
      sums[ty * tw + tx] = (sums[ty * tw + tx] ?? 0) + (grey[y * width + x] ?? 0);
      counts[ty * tw + tx] = (counts[ty * tw + tx] ?? 0) + 1;
    }
  }
  const out = new Uint8Array(tw * th);
  for (let k = 0; k < out.length; k++) out[k] = Math.round((sums[k] ?? 0) / Math.max(1, counts[k] ?? 0));
  return { grey: out, width: tw, height: th };
}

const HALF_OF_GREY = Uint16Array.from({ length: 256 }, (_, value) => halfFloatBits(value / 255));

/** A made-up number per film, so each file's UIDs are its own. */
const UID_OF: Record<FilmKind, string> = {
  front: '187160935412290345671200016901',
  side: '187160935412290345671200016902',
  'bend-left': '187160935412290345671200016903',
  'bend-right': '187160935412290345671200016904',
};

function sampleFilm(kind: FilmKind): SampleFilm {
  const facts = memo(() => (SIZED_BY_FILE.includes(kind) ? dicomFacts(kind) : pngFacts(kind)));
  const file = memo(() => {
    if (!SIZED_BY_FILE.includes(kind)) return pngOf(kind);
    const windowing = percentileWindow(storedGrey(kind));
    return writeDicom({
      grey: storedGrey(kind),
      columns: FILM_WIDTH,
      rows: FILM_HEIGHT,
      mmPerPx: MM_PER_PX,
      viewPosition: kind === 'side' ? 'LL' : 'AP',
      windowing,
      uid: UID_OF[kind],
    });
  });
  const thumbnail = memo(() => {
    const small = thumbnailGrey(storedGrey(kind), FILM_WIDTH, FILM_HEIGHT);
    return writePng(small.grey, small.width, small.height);
  });
  const pixels = memo(() => Uint16Array.from(storedGrey(kind), (value) => HALF_OF_GREY[value] ?? 0));
  return {
    kind,
    get file() {
      return file().slice().buffer;
    },
    get facts() {
      return structuredClone(facts());
    },
    get thumbnail() {
      return thumbnail().slice();
    },
    pixels: () => pixels().slice(),
  };
}

/** The four films, by kind. */
export const SAMPLE_FILMS: Record<FilmKind, SampleFilm> = {
  front: sampleFilm('front'),
  side: sampleFilm('side'),
  'bend-left': sampleFilm('bend-left'),
  'bend-right': sampleFilm('bend-right'),
};
