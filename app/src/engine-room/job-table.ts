// Every engine job: its name, which worker runs it, and its input and
// output types. FROZEN: changes only through a contract-change pull
// request.
//
// A job's input and output cross `postMessage` as the plain objects the
// engine reads and writes, so a wrong payload is a compile error here
// rather than a refusal in the worker. Each job has exactly one handler,
// in workers/<film|spine>-worker/jobs/<job name>.ts.
//
// Every point is in original film pixels. Byte fields are fresh
// `Uint8Array`s; float payloads inside them are little-endian.
//
// A film's desk holds its pixels as they were opened, described by the
// `values` in its facts. The case's record of the film can differ only in
// its brightness flip (`values.inverted`, which the user's flip toggles)
// and its blanked boxes. Every job that takes the record reads the pixels
// through it: flipped where the record's `values.inverted` differs from
// the desk's, blanked boxes zero (`PxRect` says which pixels a box
// covers).

import type {
  AnswerKey,
  AnswerKeyComparison,
  BoundaryWandInput,
  BoundaryWandOutput,
  CaseFileV1,
  ChainAnswer,
  ChainOptions,
  CheckFinding,
  Film,
  FilmChain,
  FilmFacts,
  FilmKind,
  FilmMarks,
  FilmPassResult,
  FitFromFilms,
  GeometryGuess,
  MadeUpRecipe,
  MadeUpSpine,
  OpenedCase,
  PinInput,
  PinOutput,
  Px,
  RegionWandInput,
  RegionWandOutput,
  ScaleResult,
  SpineFit,
  ToolPreview,
  ToolPreviewInput,
  TrainedPassResult,
  ReadingsInput,
  SpineReadings,
} from '@/shapes';
import type { WorkerRole } from './messages';

/**
 * A film just read: the one job output the worker puts together itself,
 * from two engine calls, so it is declared here rather than generated.
 */
export interface OpenedFilm {
  facts: FilmFacts;
  /** An 8-bit greyscale PNG, longest side at most 256 pixels, for the film slots. */
  thumbnail: Uint8Array;
}

/**
 * Job name → input and output. A film worker holds one film, so its jobs
 * need not say which; `null` means the job needs nothing more.
 */
export interface JobTable {
  // ---- Film worker: one per film, holding that film's FilmDesk.

  /**
   * Read a film; the desk keeps the pixels. `saved` is `null` for a dropped
   * file (PNG, JPEG or DICOM, judged by the bytes alone). For a case PNG it
   * is the film's saved record: the PNG must match `saved.image`, and the
   * desk takes the source, kept tags, values and flags from the record, so
   * its pixels are exactly what `saved.values` describes.
   */
  'open-film': { input: { kind: FilmKind; bytes: ArrayBuffer; saved: Film | null }; output: OpenedFilm };
  /**
   * The normalised film for the GPU: half-float bits, `facts.width` × `facts.height`,
   * as opened. A later brightness flip is the viewer's to show.
   */
  'display-pixels': { input: null; output: Uint16Array };
  /**
   * The film's stored image for the case file: the pixels read through
   * `film`, so the PNG's values are exactly what `film.values` describes,
   * blanked boxes zero. A desk opened from a case PNG and its unchanged
   * record writes the same pixels back.
   */
  'encode-png': { input: { film: Film }; output: Uint8Array };
  'sense-geometry': { input: null; output: GeometryGuess };
  'scale-from-ruler': { input: { a: Px; b: Px; mm: number }; output: ScaleResult };
  /**
   * A round object of known diameter: `near` is where the user pressed, at
   * its centre, and `edge` where they let go, on its rim, which gives the
   * circle fit its starting radius.
   */
  'scale-from-known-object': { input: { near: Px; edge: Px; diameterMm: number }; output: ScaleResult };
  /**
   * Stages 1 to 3 on this film, stepped, read through the film's record:
   * its geometry, its brightness, and its blanked boxes, which are left
   * out of every map.
   */
  'film-pass': { input: { film: Film }; output: FilmPassResult };
  /**
   * This film's solved chain turned into marks. New vertebrae take ids
   * from `firstVertebraId` upwards, so ids stay unique across the case.
   */
  'place-points': { input: { chain: FilmChain; firstVertebraId: number }; output: FilmMarks };
  /**
   * The trained finder on this front film, stepped, read through the
   * film's record: its facing, its brightness and its blanked boxes. It
   * finds, names and settles the vertebrae on its own, with no chain pass
   * after it. New vertebrae take ids from `firstVertebraId` upwards, one
   * each, never more than seventeen.
   */
  'trained-pass': { input: { film: Film; firstVertebraId: number }; output: TrainedPassResult };
  'tool-preview': { input: ToolPreviewInput; output: ToolPreview };
  'region-wand': { input: RegionWandInput; output: RegionWandOutput };
  'boundary-wand': { input: BoundaryWandInput; output: BoundaryWandOutput };
  'resolve-pins': { input: PinInput; output: PinOutput };

  // ---- Spine worker: exactly one.

  /** Stages 4 to 6 over every film's strip, stepped. */
  'chain-pass': { input: { results: FilmPassResult[]; options: ChainOptions }; output: ChainAnswer };
  'check-case': { input: CaseFileV1; output: CheckFinding[] };
  /** The `.spinecase` zip. The case is checked strictly before anything is written. */
  'pack-case': { input: OpenedCase; output: Uint8Array };
  'unpack-case': { input: { bytes: ArrayBuffer }; output: OpenedCase };
  'make-up-spine': { input: MadeUpRecipe; output: MadeUpSpine };
  'compare-with-answer-key': {
    input: { marks: FilmMarks[]; key: AnswerKey };
    output: AnswerKeyComparison;
  };
  'fit-spine': { input: FitFromFilms; output: SpineFit };
  'measure-spine': { input: ReadingsInput; output: SpineReadings };
}

export type JobName = keyof JobTable;
export type JobInput<J extends JobName> = JobTable[J]['input'];
export type JobOutput<J extends JobName> = JobTable[J]['output'];

/** How the crew runs a job. */
export interface JobRules {
  /** The worker that runs it. */
  worker: WorkerRole;
  /** A stepped pass: it reports progress and watch frames, and can be cancelled. */
  stepped: boolean;
  /**
   * At most one waits in the queue; a newer one replaces it (hover
   * previews). A replaced call rejects with `pass-cancelled`.
   */
  replaceable: boolean;
}

export const JOBS: { readonly [J in JobName]: JobRules } = {
  'open-film': { worker: 'film', stepped: false, replaceable: false },
  'display-pixels': { worker: 'film', stepped: false, replaceable: false },
  'encode-png': { worker: 'film', stepped: false, replaceable: false },
  'sense-geometry': { worker: 'film', stepped: false, replaceable: false },
  'scale-from-ruler': { worker: 'film', stepped: false, replaceable: false },
  'scale-from-known-object': { worker: 'film', stepped: false, replaceable: false },
  'film-pass': { worker: 'film', stepped: true, replaceable: false },
  'place-points': { worker: 'film', stepped: false, replaceable: false },
  'trained-pass': { worker: 'film', stepped: true, replaceable: false },
  'tool-preview': { worker: 'film', stepped: false, replaceable: true },
  'region-wand': { worker: 'film', stepped: false, replaceable: false },
  'boundary-wand': { worker: 'film', stepped: false, replaceable: false },
  'resolve-pins': { worker: 'film', stepped: false, replaceable: false },
  'chain-pass': { worker: 'spine', stepped: true, replaceable: false },
  'check-case': { worker: 'spine', stepped: false, replaceable: true },
  'pack-case': { worker: 'spine', stepped: false, replaceable: false },
  'unpack-case': { worker: 'spine', stepped: false, replaceable: false },
  'make-up-spine': { worker: 'spine', stepped: false, replaceable: false },
  'compare-with-answer-key': { worker: 'spine', stepped: false, replaceable: true },
  'fit-spine': { worker: 'spine', stepped: false, replaceable: true },
  'measure-spine': { worker: 'spine', stepped: false, replaceable: true },
};
