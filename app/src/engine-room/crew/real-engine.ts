// The EngineClient screens use in production: every call is one or more
// jobs on the crew, each over the films its answer depends on, so
// replacing or retiring a film stops exactly the calls that were about it.

import type {
  AnswerKey,
  AnswerKeyComparison,
  AutoPassInput,
  AutoPassResult,
  BoundaryWandInput,
  BoundaryWandOutput,
  CaseFileV1,
  CheckFinding,
  EngineInfo,
  Film,
  FilmKind,
  FilmMarks,
  FilmPng,
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
  ToolPreview,
  ToolPreviewInput,
} from '@/shapes';
import type { EngineClient, RunOptions } from '../engine-client';
import type { JobInput, JobName, JobOutput, OpenedFilm } from '../job-table';
import { conductAutoPass } from './auto-pass-conductor';
import type { Crew } from './crew';

export class RealEngine implements EngineClient {
  readonly #crew: Crew;

  constructor(crew: Crew) {
    this.#crew = crew;
  }

  info(): Promise<EngineInfo> {
    return this.#crew.info();
  }

  openFilm(kind: FilmKind, bytes: ArrayBuffer, saved: Film | null): Promise<OpenedFilm> {
    return this.#crew.openFilm(kind, bytes, saved);
  }

  displayPixels(kind: FilmKind): Promise<Uint16Array> {
    return this.#onFilm(kind, 'display-pixels', null);
  }

  retireFilm(kind: FilmKind): void {
    this.#crew.retireFilm(kind);
  }

  senseGeometry(kind: FilmKind): Promise<GeometryGuess> {
    return this.#onFilm(kind, 'sense-geometry', null);
  }

  scaleFromRuler(kind: FilmKind, a: Px, b: Px, mm: number): Promise<ScaleResult> {
    return this.#onFilm(kind, 'scale-from-ruler', { a, b, mm });
  }

  scaleFromKnownObject(kind: FilmKind, near: Px, edge: Px, diameterMm: number): Promise<ScaleResult> {
    return this.#onFilm(kind, 'scale-from-known-object', { near, edge, diameterMm });
  }

  runAutoPass(input: AutoPassInput, run?: RunOptions): Promise<AutoPassResult> {
    return conductAutoPass(this.#crew, input, run);
  }

  previewTool(kind: FilmKind, input: ToolPreviewInput): Promise<ToolPreview> {
    return this.#onFilm(kind, 'tool-preview', input);
  }

  regionWand(kind: FilmKind, input: RegionWandInput): Promise<RegionWandOutput> {
    return this.#onFilm(kind, 'region-wand', input);
  }

  boundaryWand(kind: FilmKind, input: BoundaryWandInput): Promise<BoundaryWandOutput> {
    return this.#onFilm(kind, 'boundary-wand', input);
  }

  resolvePins(kind: FilmKind, input: PinInput): Promise<PinOutput> {
    return this.#onFilm(kind, 'resolve-pins', input);
  }

  checkCase(c: CaseFileV1): Promise<CheckFinding[]> {
    return this.#onSpine('check-case', c);
  }

  saveCase(c: CaseFileV1): Promise<Uint8Array> {
    const scope = this.#crew.scope(c.films.map((film) => film.kind));
    return scope.settle(async () => {
      const films = await Promise.all(
        c.films.map(async (film): Promise<FilmPng> => ({ kind: film.kind, png: await this.#crew.run('encode-png', { film }, { scope, film: film.kind }) })),
      );
      return this.#crew.run('pack-case', { case: c, films }, { scope });
    });
  }

  openCase(bytes: ArrayBuffer): Promise<OpenedCase> {
    return this.#onSpine('unpack-case', { bytes });
  }

  makeUpSpine(recipe: MadeUpRecipe): Promise<MadeUpSpine> {
    return this.#onSpine('make-up-spine', recipe);
  }

  compareWithAnswerKey(marks: FilmMarks[], key: AnswerKey): Promise<AnswerKeyComparison> {
    return this.#onSpine('compare-with-answer-key', { marks, key });
  }

  /** Closes every worker. The screens never need it; tests and a page being torn down do. */
  stop(): void {
    this.#crew.stop();
  }

  #onFilm<J extends JobName>(kind: FilmKind, job: J, input: JobInput<J>): Promise<JobOutput<J>> {
    const scope = this.#crew.scope([kind]);
    return scope.settle(() => this.#crew.run(job, input, { scope, film: kind }));
  }

  #onSpine<J extends JobName>(job: J, input: JobInput<J>): Promise<JobOutput<J>> {
    const scope = this.#crew.scope([]);
    return scope.settle(() => this.#crew.run(job, input, { scope }));
  }
}
