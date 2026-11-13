// The EngineClient: the only engine import screens use. FROZEN: changes
// only through a contract-change pull request.
//
// The real engine (the crew of workers) and the stand-in both implement
// it. Screens never see workers, messages or wasm.
//
// Rules every implementation keeps:
// - Every promise rejects with an `EngineFailure`, whose `error.code` the
//   app words through words/faults.ts. Nothing else is ever thrown.
// - Film calls name the film by kind; each film lives in its own worker.
// - The engine keeps nothing but each film's pixels and caches. Every edit
//   request and the auto pass carry the case version they were computed
//   from, and every answer echoes that version, so the case store can
//   drop an answer that arrives after its film changed, or that uses a
//   vertebra id the case may have given out since (the rules in
//   case-state/case-edits.ts).
// - Every point is in original film pixels.

import type {
  AutoPassInput,
  AutoPassResult,
  BoundaryWandInput,
  BoundaryWandOutput,
  CaseFileV1,
  CheckFinding,
  EngineError,
  EngineInfo,
  Film,
  FilmKind,
  FilmMarks,
  FitFromFilms,
  GeometryGuess,
  MadeUpRecipe,
  MadeUpSpine,
  AnswerKey,
  AnswerKeyComparison,
  OpenedCase,
  PinInput,
  PinOutput,
  Px,
  RegionWandInput,
  RegionWandOutput,
  ScaleResult,
  SpineFit,
  Stage,
  ToolPreview,
  ToolPreviewInput,
  WatchFrame,
  ReadingsInput,
  SpineReadings,
} from '@/shapes';
import type { OpenedFilm } from './job-table';

export interface EngineClient {
  /** The engine's version, shapes fingerprint and case format. */
  info(): Promise<EngineInfo>;

  /**
   * Read a film's bytes: a dropped file (PNG, JPEG or DICOM, judged by the
   * bytes alone) with `saved` null, or a case PNG with its saved record.
   * For a kind that is already open it replaces that film, and the calls
   * still waiting or running on the old one reject with `pass-cancelled`
   * (docs/contracts/engine-api.md says which, and what the screen does
   * with them). If it fails, the film that was open stays open and
   * unchanged.
   */
  openFilm(kind: FilmKind, bytes: ArrayBuffer, saved: Film | null): Promise<OpenedFilm>;
  /** The film for the GPU: half-float bits, width × height from its facts. */
  displayPixels(kind: FilmKind): Promise<Uint16Array>;
  /**
   * Forget a film: its worker is closed and its bytes dropped. The calls
   * still waiting or running on it reject with `pass-cancelled`
   * (docs/contracts/engine-api.md says which, and what the screen does
   * with them).
   */
  retireFilm(kind: FilmKind): void;

  senseGeometry(kind: FilmKind): Promise<GeometryGuess>;
  scaleFromRuler(kind: FilmKind, a: Px, b: Px, mm: number): Promise<ScaleResult>;
  /** A round object: pressed at its centre (`near`), released on its rim (`edge`). */
  scaleFromKnownObject(kind: FilmKind, near: Px, edge: Px, diameterMm: number): Promise<ScaleResult>;

  /**
   * Film passes in parallel, each read through its film's record; the
   * chain pass; then points film by film, new ids counting up from
   * `input.firstVertebraId` across the films.
   *
   * The answer's `caseVersion` is `input.caseVersion`, stamped by the
   * client itself: the jobs between workers never reach the case store,
   * so they carry no version. The screens apply the answer as one action:
   * one `replace-marks` edit per entry of `marks`, each with cause
   * `{ by: 'auto-pass' }` and `basedOn: caseVersion`, in order; each is
   * kept or dropped on its own. When only some are kept, they show
   * `twoFilms` only if all were, and only the kept films' warnings.
   * Replacing or retiring one of its films rejects it as a whole with
   * `pass-cancelled`.
   */
  runAutoPass(input: AutoPassInput, run?: RunOptions): Promise<AutoPassResult>;

  /**
   * What a click would do here, before any click. A newer call replaces a
   * waiting one; a replaced call rejects with `pass-cancelled`, which the
   * screen lets go by.
   */
  previewTool(kind: FilmKind, input: ToolPreviewInput): Promise<ToolPreview>;
  regionWand(kind: FilmKind, input: RegionWandInput): Promise<RegionWandOutput>;
  boundaryWand(kind: FilmKind, input: BoundaryWandInput): Promise<BoundaryWandOutput>;
  resolvePins(kind: FilmKind, input: PinInput): Promise<PinOutput>;

  /**
   * The case checks. A newer call replaces a waiting one; a replaced call
   * rejects with `pass-cancelled`, which the screen lets go by.
   */
  checkCase(c: CaseFileV1): Promise<CheckFinding[]>;
  /**
   * The `.spinecase` bytes: each film's PNG, then the zip. Replacing or
   * retiring one of its films rejects it as a whole with `pass-cancelled`,
   * which is not a stopped search: the save sheet starts the save again.
   */
  saveCase(c: CaseFileV1): Promise<Uint8Array>;
  /**
   * Read and check a `.spinecase`; it opens no film. The screen then opens
   * each film through `openFilm` with its saved record.
   */
  openCase(bytes: ArrayBuffer): Promise<OpenedCase>;

  makeUpSpine(recipe: MadeUpRecipe): Promise<MadeUpSpine>;
  /**
   * The marks against a made-up spine's answer key. A newer call replaces a
   * waiting one; a replaced call rejects with `pass-cancelled`, which the
   * screen lets go by.
   */
  compareWithAnswerKey(marks: FilmMarks[], key: AnswerKey): Promise<AnswerKeyComparison>;
  /** Fit one front film or a front and side pair. The caller drops stale answers. */
  fitSpine(input: FitFromFilms): Promise<SpineFit>;
  /** Measure a fitted spine. A newer call replaces a waiting one; the caller drops stale answers. */
  measureSpine(input: ReadingsInput): Promise<SpineReadings>;
}

/** Live reports from a long call, and a way to stop it. */
export interface RunOptions {
  /** At most ten a second. */
  onProgress?(progress: { stage: Stage; fraction: number }): void;
  /**
   * Every frame is kept except two kinds, which may be thinned to thirty a
   * second, newest kept: `joint-progress` frames, and `evidence` frames
   * with `final: false` whose `map` is not `wavefront`. Each film's
   * frames, and the chain pass's, arrive in the order they were emitted.
   */
  onFrame?(frame: WatchFrame): void;
  /** Aborting rejects the call with `pass-cancelled`. */
  signal?: AbortSignal;
}

/** The one thing an EngineClient call rejects with. */
export class EngineFailure extends Error {
  readonly error: EngineError;

  constructor(error: EngineError) {
    super(error.detail === null ? `${error.code}: ${error.message}` : `${error.code}: ${error.message} (${error.detail})`);
    this.name = 'EngineFailure';
    this.error = error;
  }
}
