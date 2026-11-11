// One EngineClient call, from the moment it is made until it settles: the
// films its answer depends on, and the one way it can be stopped early.
//
// A call is stopped by its AbortSignal, by a newer call replacing it, or
// by one of its films being replaced or retired. Stopping it rejects the
// call at once with the stopping error, and every job it still has
// waiting or running is withdrawn, so nothing it started reaches the
// screens afterwards.

import type { EngineError, FilmKind } from '@/shapes';
import { EngineFailure } from '../engine-client';

export class CallScope {
  readonly films: ReadonlySet<FilmKind>;
  #stopped: EngineError | null = null;
  #settled = false;
  #listeners = new Set<(error: EngineError) => void>();
  readonly #release: () => void;

  constructor(films: Iterable<FilmKind>, release: () => void = () => undefined) {
    this.films = new Set(films);
    this.#release = release;
  }

  /** Why the call was stopped, or null while it may still answer. */
  get stopped(): EngineError | null {
    return this.#stopped;
  }

  /** Whether the call has answered or been stopped. */
  get settled(): boolean {
    return this.#settled;
  }

  /** Stops the call with `error`, unless it has already settled. */
  stop(error: EngineError): void {
    if (this.#settled) return;
    this.#stopped = error;
    this.#settled = true;
    const listeners = [...this.#listeners];
    this.#listeners.clear();
    this.#release();
    for (const listener of listeners) listener(error);
  }

  /** Runs `listener` if the call is stopped. Returns a way to stop listening. */
  onStop(listener: (error: EngineError) => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** The call has answered: nothing can stop it now. */
  finish(): void {
    if (this.#settled) return;
    this.#settled = true;
    this.#listeners.clear();
    this.#release();
  }

  /**
   * The call's promise: `body`'s answer, unless the call is stopped first,
   * when it rejects at once with the stopping error. If `body` fails, the
   * call is stopped with its error, which withdraws the rest of its jobs.
   */
  settle<T>(body: () => Promise<T>): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      if (this.#stopped !== null) {
        reject(new EngineFailure(this.#stopped));
        return;
      }
      this.onStop((error) => {
        reject(new EngineFailure(error));
      });
      body().then(
        (answer) => {
          if (this.#settled) return;
          this.finish();
          resolve(answer);
        },
        (thrown: unknown) => {
          const error = engineErrorOf(thrown);
          if (!this.#settled) this.stop(error);
          reject(new EngineFailure(error));
        },
      );
    });
  }
}

/** The error a call that was replaced, or whose film was, rejects with. */
export function passCancelled(why: string): EngineError {
  return { code: 'pass-cancelled', message: 'the call was cancelled', detail: why };
}

/** Whatever a call failed with, as the EngineError it rejects with. */
export function engineErrorOf(thrown: unknown): EngineError {
  if (thrown instanceof EngineFailure) return thrown.error;
  return { code: 'engine-crashed', message: 'the engine room failed unexpectedly', detail: String(thrown) };
}
