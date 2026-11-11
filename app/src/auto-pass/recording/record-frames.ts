// Watch frames in, a recording out.
//
// The recorder keeps what the pass shows and lets the rest go by: of the
// evidence maps, only the finished `phase-edge`, `ridge` and `ring`, plus
// every `wavefront` tile; no `template-scored` or `joint-progress` frame;
// of the outlines, only the final ones. The watch-frame contract keeps
// each film's frames in the order they were emitted, and lets the films'
// frames interleave: each film's part depends on its own frames alone,
// read in that order, which the beat readiness rules and the strip's
// assembly rely on. It reads nothing but the frames and the pass's
// answer, and makes up nothing they did not carry.
//
// A beat is ready once every film of the pass has sent everything the
// beat shows, and every beat before it is ready. Only frames make a beat
// ready, never the pass's answer: a film whose frames stopped short holds
// the clock even after the pass returns, and whoever shows the result
// then has to offer it without waiting for the clock.

import type { AutoPassResult, FilmKind, MapKind, WatchFrame } from '@/shapes';
import { BEATS, type BeatId } from '../beats';
import { SHOWN_MAPS, type Barcode, type FilmRecording, type FilmStrip, type PassRecording, type ShownMap } from './pass-recording';
import { stripImage } from './strip-image';
import { tileImage } from './tile-image';

/** An empty recording for a pass over these films, in the order of the pass's input. */
export function startRecording(films: readonly FilmKind[]): PassRecording {
  return { films: films.map(emptyFilm), result: null };
}

/** The recording with one more frame. A frame for a film outside the pass changes nothing. */
export function recordFrame(recording: PassRecording, frame: WatchFrame): PassRecording {
  const index = recording.films.findIndex((part) => part.film === frame.film);
  const part = recording.films[index];
  if (part === undefined) return recording;
  const next = withFrame(part, frame);
  if (next === part) return recording;
  const films = recording.films.slice();
  films[index] = next;
  return { ...recording, films };
}

/** A whole recording from a list of frames. */
export function recordFrames(films: readonly FilmKind[], frames: readonly WatchFrame[]): PassRecording {
  return frames.reduce(recordFrame, startRecording(films));
}

/** The recording with the pass's answer: the marks, the two-film check and the warnings. */
export function recordResult(recording: PassRecording, result: AutoPassResult): PassRecording {
  return { ...recording, result };
}

/** One film's part of the recording, or null when the pass did not run that film. */
export function filmRecording(recording: PassRecording, film: FilmKind): FilmRecording | null {
  return recording.films.find((part) => part.film === film) ?? null;
}

/** A film's strip, once its shape, every row and its profile have arrived. */
export function filmStrip(part: FilmRecording): FilmStrip | null {
  const { stripShape: shape, strip: image, barcode } = part;
  if (shape === null || image === null || barcode === null) return null;
  if (barcode.values.length !== shape.centres.length) return null;
  return { shape, image, barcode };
}

/** What each beat shows, by film. */
const BEAT_NEEDS: Record<BeatId, (part: FilmRecording) => boolean> = {
  // A ring map comes before the centre line starts, if the film has one.
  evidence: (part) => part.maps['phase-edge'] !== undefined && part.maps.ridge !== undefined && part.stagesStarted.includes('centre-line'),
  // Every wavefront tile comes before the coarse line.
  'centre-line': (part) => part.endZones !== null && part.coarseLine !== null && part.refinedLine !== null,
  unroll: (part) => filmStrip(part) !== null,
  // Every sweep frame comes before the chain locks.
  search: (part) => part.locked !== null,
  'lock-in': (part) => part.locked !== null && part.named !== null,
  confidence: (part) => part.confidence !== null && part.outlines !== null,
};

/** How many beats, from the first, have all their data: 0 to 6. */
export function readyBeats(recording: PassRecording): number {
  if (recording.films.length === 0) return 0;
  const ready = BEATS.findIndex((beat) => !recording.films.every(BEAT_NEEDS[beat.id]));
  return ready === -1 ? BEATS.length : ready;
}

function emptyFilm(film: FilmKind): FilmRecording {
  return {
    film,
    stagesStarted: [],
    maps: {},
    endZones: null,
    wavefront: [],
    coarseLine: null,
    refinedLine: null,
    width: null,
    stripShape: null,
    stripBlocks: [],
    strip: null,
    barcode: null,
    sweeps: [],
    locked: null,
    named: null,
    confidence: null,
    outlines: null,
    warnings: [],
  };
}

const SHOWN: readonly MapKind[] = SHOWN_MAPS;

function isShown(map: MapKind): map is ShownMap {
  return SHOWN.includes(map);
}

function withFrame(part: FilmRecording, frame: WatchFrame): FilmRecording {
  switch (frame.kind) {
    case 'stage-started':
      return part.stagesStarted.includes(frame.stage) ? part : { ...part, stagesStarted: [...part.stagesStarted, frame.stage] };
    case 'evidence': {
      if (frame.map === 'wavefront') {
        const image = tileImage(frame.tile);
        if (image === null) return part;
        const others = part.wavefront.filter((tile) => tile.step !== frame.step);
        return { ...part, wavefront: [...others, { step: frame.step, image }].sort((p, q) => p.step - q.step) };
      }
      if (!frame.final || !isShown(frame.map)) return part;
      const image = tileImage(frame.tile);
      return image === null ? part : { ...part, maps: { ...part.maps, [frame.map]: image } };
    }
    case 'end-zones':
      return { ...part, endZones: { top: frame.top, bottom: frame.bottom, topCropped: frame.topCropped, bottomCropped: frame.bottomCropped } };
    case 'centre-line':
      return frame.refined ? { ...part, refinedLine: frame.points } : { ...part, coarseLine: frame.points };
    case 'width':
      return { ...part, width: frame.samples };
    case 'strip-shape':
      return withWholeStrip({ ...part, stripShape: { centres: frame.centres, normals: frame.normals, columnPx: frame.columnPx, columns: frame.columns } });
    case 'strip-rows':
      return part.strip !== null ? part : withWholeStrip({ ...part, stripBlocks: [...part.stripBlocks, frame] });
    case 'barcode': {
      const barcode = decodeBarcode(frame.values, frame.highPass);
      return barcode === null ? part : { ...part, barcode };
    }
    case 'sweep':
      return { ...part, sweeps: [...part.sweeps, { level: frame.level, heat: frame.heat, tentative: frame.tentative }] };
    case 'template-scored':
    case 'joint-progress':
      return part;
    case 'chain-locked':
      return { ...part, locked: frame.bodies };
    case 'levels-named':
      return { ...part, named: { naming: frame.naming, levels: frame.levels } };
    case 'confidence':
      return { ...part, confidence: frame.levels };
    case 'outlines':
      return frame.final ? { ...part, outlines: frame.quads } : part;
    case 'warning':
      return { ...part, warnings: [...part.warnings, frame.warning] };
  }
}

/** Builds the strip bitmap as soon as the shape and every block are in. */
function withWholeStrip(part: FilmRecording): FilmRecording {
  if (part.strip !== null || part.stripShape === null || part.stripBlocks.length === 0) return part;
  const image = stripImage(part.stripBlocks, part.stripShape.centres.length, part.stripShape.columns);
  return image === null ? part : { ...part, strip: image, stripBlocks: [] };
}

/** Two little-endian `f32` lists of the same length, or null when the bytes are not that. */
function decodeBarcode(values: Uint8Array, highPass: Uint8Array): Barcode | null {
  if (values.length % 4 !== 0 || highPass.length !== values.length) return null;
  return { values: littleEndianFloats(values), highPass: littleEndianFloats(highPass) };
}

function littleEndianFloats(bytes: Uint8Array): Float32Array {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return Float32Array.from({ length: bytes.length / 4 }, (_, i) => view.getFloat32(i * 4, true));
}
