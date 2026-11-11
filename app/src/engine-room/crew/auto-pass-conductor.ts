// The auto pass, conducted from the main thread, since workers never talk
// to each other:
// 1. a film pass on every film's worker at once;
// 2. their small results to the spine worker's chain pass;
// 3. each film's worker places its points, film by film, new vertebra
//    ids counting up across the films from `input.firstVertebraId`.
//
// It is one call over every film it runs: replacing or retiring any of
// them, a failure in any job, or the signal stops all of it at once, and
// no frame reaches the screen after that. Frames pass straight through,
// already thinned by each worker; the film passes run side by side, so
// their frames may interleave. Progress from the parallel film passes is
// joined into one, and sent at most ten times a second.

import type { AutoPassInput, AutoPassResult, FilmKind, FilmMarks, PassWarning, Stage, WatchFrame } from '@/shapes';
import { EngineFailure, type RunOptions } from '../engine-client';
import type { Crew } from './crew';

const STAGES: readonly Stage[] = ['evidence', 'centre-line', 'strip', 'chain', 'naming', 'confidence'];
export const PROGRESS_GAP_MS = 100;

export function conductAutoPass(crew: Crew, input: AutoPassInput, run: RunOptions = {}): Promise<AutoPassResult> {
  const scope = crew.scope(
    input.films.map((film) => film.kind),
    run.signal,
  );
  const progress = new JoinedProgress(input.films.map((film) => film.kind), (stage, fraction) => run.onProgress?.({ stage, fraction }));
  scope.onStop(() => {
    progress.stop();
  });
  const frame = (watched: WatchFrame) => {
    run.onFrame?.(watched);
  };

  return scope.settle(async () => {
    const results = await Promise.all(
      input.films.map((film) =>
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
      { results, options: input.options },
      {
        scope,
        frame,
        progress: (stage, fraction) => {
          progress.chain(stage, fraction);
        },
      },
    );

    const marks: FilmMarks[] = [];
    let firstVertebraId = input.firstVertebraId;
    for (const film of input.films) {
      const filmChain = chain.films.find((answer) => answer.film === film.kind);
      if (filmChain === undefined) {
        throw new EngineFailure({ code: 'engine-out-of-sync', message: 'the chain pass gave no chain for a film it was given', detail: film.kind });
      }
      const placed = await crew.run('place-points', { chain: filmChain, firstVertebraId }, { scope, film: film.kind });
      for (const vertebra of placed.vertebrae) firstVertebraId = Math.max(firstVertebraId, vertebra.id + 1);
      marks.push(placed);
    }
    progress.stop();

    return {
      caseVersion: input.caseVersion,
      marks,
      twoFilms: chain.twoFilms,
      warnings: onceEach([...results.flatMap((result) => result.warnings), ...chain.films.flatMap((answer) => answer.warnings)]),
    };
  });
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
 * One progress line from several passes. While the film passes run, the
 * stage is the furthest-behind film's, and the fraction the films' mean
 * through that stage (a film past it counts as through). Then the chain
 * pass's own. At most one report per `PROGRESS_GAP_MS`, newest kept.
 */
export class JoinedProgress {
  readonly #films = new Map<FilmKind, { stage: number; fraction: number }>();
  readonly #send: (stage: Stage, fraction: number) => void;
  #last = Number.NEGATIVE_INFINITY;
  #held: { stage: Stage; fraction: number } | null = null;
  #timer: ReturnType<typeof setTimeout> | null = null;
  #stopped = false;

  constructor(films: FilmKind[], send: (stage: Stage, fraction: number) => void) {
    for (const film of films) this.#films.set(film, { stage: 0, fraction: 0 });
    this.#send = send;
  }

  film(kind: FilmKind, stage: Stage, fraction: number): void {
    this.#films.set(kind, { stage: STAGES.indexOf(stage), fraction });
    const states = [...this.#films.values()];
    const behind = Math.min(...states.map((state) => state.stage));
    const through = states.map((state) => (state.stage > behind ? 1 : state.fraction));
    this.#offer(STAGES[behind] ?? 'evidence', through.reduce((sum, value) => sum + value, 0) / states.length);
  }

  chain(stage: Stage, fraction: number): void {
    this.#offer(stage, fraction);
  }

  stop(): void {
    this.#stopped = true;
    if (this.#timer !== null) clearTimeout(this.#timer);
    this.#timer = null;
    this.#held = null;
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
