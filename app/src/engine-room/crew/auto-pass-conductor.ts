// The auto pass, conducted from the main thread, since workers never talk
// to each other:
// 1. a film pass on every film's worker at once;
// 2. their small results to the spine worker's chain pass;
// 3. each film's worker places its points, film by film, new vertebra
//    ids counting up across the films from `input.firstVertebraId`.
//
// When `input.frontFinder` is 'trained' and the front film is among the
// films, the front film's worker runs the trained finder ('trained-pass')
// instead, side by side with the other films' film passes. It finds, names
// and settles its vertebrae on its own, so the chain pass and
// 'place-points' run for the other films only, and not at all when there
// are none. `twoFilms` is the chain pass's, which pairs a front film with
// a side film only when it is given both, so it is null whenever the
// trained finder reads the front film or the chain pass does not run.
// Marks come back in the order of `input.films` either way, and the
// warnings of every pass are joined, each once.
//
// Vertebra ids must rise film after film in the order of `input.films`:
// the case store judges each film's edit against the case the edits
// before it left, so a film whose ids lay below an earlier film's would
// hold ids under the case's next free id and be dropped. The trained pass
// needs its first id when it starts, before any other film has placed its
// points, so it starts at `input.firstVertebraId` and gives its vertebrae
// consecutive ids from there. When its turn comes in the order, its ids
// are all moved up together to start at the next free id, as a placed
// film's would. With the front film first, as the case keeps its films,
// nothing moves; with it later, no id is reserved and none is skipped.
//
// It is one call over every film it runs: replacing or retiring any of
// them, a failure in any job, or the signal stops all of it at once, and
// no frame reaches the screen after that. Frames pass straight through,
// already thinned by each worker; the passes run side by side, so their
// frames may interleave.
//
// Progress from the passes running side by side is joined into one line,
// sent at most ten times a second: the stage of the part furthest behind,
// and the parts' mean through that stage. Each hand-built film is a part
// (its film pass from evidence to strip, then the chain pass, which stands
// for all of them, from chain to confidence). The trained pass is a part
// of its own: evidence while it reads the film, then chain, naming and
// confidence. It has no centre-line or strip stage, so once it is past
// evidence it counts as through those, and the line follows the film
// passes; while it is still reading, the line stays at evidence even if
// the chain pass has begun. Once it has finished, it counts as through
// every stage, and the line is the other films' alone.

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
  const trained = input.frontFinder === 'trained' ? (input.films.find((film) => film.kind === 'front') ?? null) : null;
  const handBuilt = input.films.filter((film) => film !== trained);
  const progress = new JoinedProgress(
    input.films.map((film) => film.kind),
    (stage, fraction) => run.onProgress?.({ stage, fraction }),
    trained?.kind ?? null,
  );
  scope.onStop(() => {
    progress.stop();
  });
  const frame = (watched: WatchFrame) => {
    run.onFrame?.(watched);
  };

  return scope.settle(async () => {
    const [found, chained] = await Promise.all([
      trained === null ? null : trainedPass(crew, trained, input.firstVertebraId, scope, frame, progress),
      handBuilt.length === 0 ? null : handBuiltPasses(crew, handBuilt, input.options, scope, frame, progress),
    ]);

    const marks: FilmMarks[] = [];
    const passWarnings: PassWarning[] = [];
    let firstVertebraId = input.firstVertebraId;
    for (const film of input.films) {
      let placed: FilmMarks;
      if (found !== null && film === trained) {
        placed = movedUp(found.marks, firstVertebraId - input.firstVertebraId);
        passWarnings.push(...found.warnings);
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

/** The trained finder on the front film, its ids from `firstVertebraId`. */
async function trainedPass(
  crew: Crew,
  film: Film,
  firstVertebraId: number,
  scope: CallScope,
  frame: (watched: WatchFrame) => void,
  progress: JoinedProgress,
): Promise<TrainedPassResult> {
  const found = await crew.run(
    'trained-pass',
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
 * The trained front film's marks with every id moved up by `by`. Moving
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
 * for every film but the trained one; while no trained pass is still
 * running, its own stage and fraction are the line. A trained pass that
 * has finished counts as through every stage. At most one report per
 * `PROGRESS_GAP_MS`, newest kept.
 */
export class JoinedProgress {
  readonly #films = new Map<FilmKind, { stage: number; fraction: number }>();
  readonly #send: (stage: Stage, fraction: number) => void;
  readonly #trained: FilmKind | null;
  #last = Number.NEGATIVE_INFINITY;
  #held: { stage: Stage; fraction: number } | null = null;
  #timer: ReturnType<typeof setTimeout> | null = null;
  #stopped = false;

  /** `trained` is the film the trained finder reads, if one is among `films`. */
  constructor(films: FilmKind[], send: (stage: Stage, fraction: number) => void, trained: FilmKind | null = null) {
    for (const film of films) this.#films.set(film, { stage: 0, fraction: 0 });
    this.#send = send;
    this.#trained = trained;
  }

  film(kind: FilmKind, stage: Stage, fraction: number): void {
    this.#films.set(kind, { stage: STAGES.indexOf(stage), fraction });
    this.#join();
  }

  chain(stage: Stage, fraction: number): void {
    for (const kind of this.#films.keys()) {
      if (kind !== this.#trained) this.#films.set(kind, { stage: STAGES.indexOf(stage), fraction });
    }
    const trained = this.#trained === null ? undefined : this.#films.get(this.#trained);
    if (trained === undefined || trained.stage === STAGES.length) this.#offer(stage, fraction);
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
