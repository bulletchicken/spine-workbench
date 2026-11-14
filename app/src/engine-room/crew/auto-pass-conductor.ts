// Each selected trained finder runs on its film worker beside the hand-built
// film passes. Only hand-built films enter the chain and point placement.
// Results and warnings are joined in input order, with trained ids moved up
// to the next free id. One scope stops every participating film together.

import type { AutoPassInput, AutoPassResult, ChainAnswer, ChainOptions, Film, FilmKind, FilmMarks, FilmPassResult, PassWarning, Stage, TrainedPassResult, WatchFrame } from '@/shapes';
import { EngineFailure, type RunOptions } from '../engine-client';
import type { CallScope } from './call-scope';
import type { Crew } from './crew';

const STAGES: readonly Stage[] = ['evidence', 'centre-line', 'strip', 'chain', 'naming', 'confidence'];
export const PROGRESS_GAP_MS = 100;

export function conductAutoPass(crew: Crew, input: AutoPassInput, run: RunOptions = {}): Promise<AutoPassResult> {
  const scope = crew.scope(
    input.films.map((film) => film.kind),
    run.signal,
  );
  const trained = input.films.filter((film) =>
    (film.kind === 'front' && input.frontFinder === 'trained') ||
    (film.kind === 'side' && input.sideFinder === 'trained'));
  const trainedKinds = new Set(trained.map((film) => film.kind));
  const handBuilt = input.films.filter((film) => !trainedKinds.has(film.kind));
  const progress = new JoinedProgress(
    input.films.map((film) => film.kind),
    (stage, fraction) => run.onProgress?.({ stage, fraction }),
    trainedKinds,
  );
  scope.onStop(() => {
    progress.stop();
  });
  const frame = (watched: WatchFrame) => {
    run.onFrame?.(watched);
  };

  return scope.settle(async () => {
    const [found, chained] = await Promise.all([
      Promise.all(trained.map((film) => trainedPass(crew, film, input.firstVertebraId, scope, frame, progress))),
      handBuilt.length === 0 ? null : handBuiltPasses(crew, handBuilt, input.options, scope, frame, progress),
    ]);

    const trainedResults = new Map(found.map((result) => [result.marks.film, result]));
    const marks: FilmMarks[] = [];
    const passWarnings: PassWarning[] = [];
    let firstVertebraId = input.firstVertebraId;
    for (const film of input.films) {
      let placed: FilmMarks;
      const trainedResult = trainedResults.get(film.kind);
      if (trainedResult !== undefined) {
        placed = movedUp(trainedResult.marks, firstVertebraId - input.firstVertebraId);
        passWarnings.push(...trainedResult.warnings);
      } else {
        const filmChain = chained?.chain.films.find((answer) => answer.film === film.kind);
        if (chained === null || filmChain === undefined) {
          throw new EngineFailure({ code: 'engine-out-of-sync', message: 'the chain pass gave no chain for a film it was given', detail: film.kind });
        }
        placed = await crew.run('place-points', { chain: filmChain, firstVertebraId }, { scope, film: film.kind });
        passWarnings.push(...(chained.results[handBuilt.indexOf(film)]?.warnings ?? []));
      }
      for (const vertebra of placed.vertebrae) firstVertebraId = Math.max(firstVertebraId, vertebra.id + 1);
      marks.push(placed);
    }
    progress.stop();

    return {
      caseVersion: input.caseVersion,
      marks,
      twoFilms: chained?.chain.twoFilms ?? null,
      warnings: onceEach([...passWarnings, ...(chained?.chain.films.flatMap((answer) => answer.warnings) ?? [])]),
    };
  });
}

/** The selected trained finder, its ids from `firstVertebraId`. */
async function trainedPass(
  crew: Crew,
  film: Film,
  firstVertebraId: number,
  scope: CallScope,
  frame: (watched: WatchFrame) => void,
  progress: JoinedProgress,
): Promise<TrainedPassResult> {
  const found = await crew.run(
    film.kind === 'side' ? 'trained-side-pass' : 'trained-pass',
    { film, firstVertebraId },
    {
      scope,
      film: film.kind,
      frame,
      progress: (stage, fraction) => {
        progress.film(film.kind, stage, fraction);
      },
    },
  );
  progress.through(film.kind);
  return found;
}

/** The hand-built stages: a film pass on each film at once, then one chain pass over them all. */
async function handBuiltPasses(
  crew: Crew,
  films: Film[],
  options: ChainOptions,
  scope: CallScope,
  frame: (watched: WatchFrame) => void,
  progress: JoinedProgress,
): Promise<{ results: FilmPassResult[]; chain: ChainAnswer }> {
  const results = await Promise.all(
    films.map((film) =>
      crew.run(
        'film-pass',
        { film },
        {
          scope,
          film: film.kind,
          frame,
          progress: (stage, fraction) => {
            progress.film(film.kind, stage, fraction);
          },
        },
      ),
    ),
  );
  const chain = await crew.run(
    'chain-pass',
    { results, options },
    {
      scope,
      frame,
      progress: (stage, fraction) => {
        progress.chain(stage, fraction);
      },
    },
  );
  return { results, chain };
}

/**
 * A trained film's marks with every id moved up by `by`. Moving
 * them all by the same amount keeps them unique and in order.
 */
function movedUp(marks: FilmMarks, by: number): FilmMarks {
  if (by === 0) return marks;
  return { ...marks, vertebrae: marks.vertebrae.map((vertebra) => ({ ...vertebra, id: vertebra.id + by })) };
}

/** Each warning once, in the order first given: a film pass's warning may come again in its chain. */
function onceEach(warnings: PassWarning[]): PassWarning[] {
  const seen = new Set<string>();
  return warnings.filter((warning) => {
    const key = JSON.stringify(warning);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

/**
 * One progress line from several passes. Each film is a part. The stage
 * is the furthest-behind part's, and the fraction the parts' mean through
 * that stage (a part past it counts as through). The chain pass speaks
 * for the hand-built films; while no trained pass is still
 * running, its own stage and fraction are the line. A trained pass that
 * has finished counts as through every stage. At most one report per
 * `PROGRESS_GAP_MS`, newest kept.
 */
export class JoinedProgress {
  readonly #films = new Map<FilmKind, { stage: number; fraction: number }>();
  readonly #send: (stage: Stage, fraction: number) => void;
  readonly #trained: ReadonlySet<FilmKind>;
  #last = Number.NEGATIVE_INFINITY;
  #held: { stage: Stage; fraction: number } | null = null;
  #timer: ReturnType<typeof setTimeout> | null = null;
  #stopped = false;

  /** `trained` contains the films read by trained finders. */
  constructor(films: FilmKind[], send: (stage: Stage, fraction: number) => void, trained: ReadonlySet<FilmKind> | FilmKind | null = null) {
    for (const film of films) this.#films.set(film, { stage: 0, fraction: 0 });
    this.#send = send;
    this.#trained = trained === null ? new Set() : typeof trained === 'string' ? new Set([trained]) : trained;
  }

  film(kind: FilmKind, stage: Stage, fraction: number): void {
    this.#films.set(kind, { stage: STAGES.indexOf(stage), fraction });
    this.#join();
  }

  chain(stage: Stage, fraction: number): void {
    for (const kind of this.#films.keys()) {
      if (!this.#trained.has(kind)) this.#films.set(kind, { stage: STAGES.indexOf(stage), fraction });
    }
    const finished = [...this.#trained].every((kind) => this.#films.get(kind)?.stage === STAGES.length);
    if (finished) this.#offer(stage, fraction);
    else this.#join();
  }

  /** The trained pass on `kind` has finished. */
  through(kind: FilmKind): void {
    this.#films.set(kind, { stage: STAGES.length, fraction: 1 });
    this.#join();
  }

  stop(): void {
    this.#stopped = true;
    if (this.#timer !== null) clearTimeout(this.#timer);
    this.#timer = null;
    this.#held = null;
  }

  #join(): void {
    const states = [...this.#films.values()];
    const behind = Math.min(...states.map((state) => state.stage));
    // Every part has finished: the answer is on its way, with nothing more to report.
    if (behind === STAGES.length) return;
    const through = states.map((state) => (state.stage > behind ? 1 : state.fraction));
    this.#offer(STAGES[behind] ?? 'evidence', through.reduce((sum, value) => sum + value, 0) / states.length);
  }

  #offer(stage: Stage, fraction: number): void {
    if (this.#stopped) return;
    const now = performance.now();
    if (now - this.#last >= PROGRESS_GAP_MS) {
      this.#last = now;
      this.#send(stage, fraction);
      return;
    }
    this.#held = { stage, fraction };
    this.#timer ??= setTimeout(
      () => {
        this.#timer = null;
        const held = this.#held;
        this.#held = null;
        if (held === null || this.#stopped) return;
        this.#last = performance.now();
        this.#send(held.stage, held.fraction);
      },
      PROGRESS_GAP_MS - (now - this.#last),
    );
  }
}
