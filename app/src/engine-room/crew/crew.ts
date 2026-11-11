// The crew: the spine worker, one film worker per open film, and the
// rules that keep each call's answer true to the films it was asked about.
//
// At start-up it boots the spine worker and one spare film worker, so the
// first film opened takes a worker that is already running. Every later
// film starts its own worker when it is opened.
//
// A call is a CallScope over the films its answer depends on. Opening a
// film of a kind that is already open replaces it: every call still
// waiting or running on the old film is stopped with `pass-cancelled`
// before the new `open-film` joins that worker's queue, so a call made
// afterwards goes to the new film. If the new film cannot be read, the
// worker keeps the old one, and so do the kept bytes. Retiring a film
// stops its calls the same way and closes its worker.

import type { EngineInfo, Film, FilmKind, Stage, WatchFrame } from '@/shapes';
import { EngineFailure } from '../engine-client';
import { JOBS, type JobInput, type JobName, type JobOutput, type OpenedFilm } from '../job-table';
import { CallScope, passCancelled } from './call-scope';
import { CrewMember, type SpawnWorker, type Ticket } from './crew-member';
import { FilmBytes } from './film-bytes';

export interface CrewSetup {
  /** Starts a worker. */
  spawn: SpawnWorker;
  /** The compiled engine, handed to every worker at boot. */
  engine(): Promise<WebAssembly.Module>;
}

/** Where a job runs and where what it says goes. */
export interface JobPlace {
  scope: CallScope;
  /** The film whose worker runs it; spine jobs leave it out. */
  film?: FilmKind;
  progress?(stage: Stage, fraction: number): void;
  frame?(frame: WatchFrame): void;
}

export class Crew {
  readonly #spawn: SpawnWorker;
  readonly #module: Promise<WebAssembly.Module>;
  readonly #spine: CrewMember;
  readonly #films = new Map<FilmKind, CrewMember>();
  readonly #bytes = new FilmBytes();
  readonly #scopes = new Set<CallScope>();
  readonly #info: Promise<EngineInfo>;
  readonly #infoLost: (failure: EngineFailure) => void;
  #spare: CrewMember | null;
  #nextId = 1;
  #stopped = false;

  constructor(setup: CrewSetup) {
    this.#spawn = setup.spawn;
    this.#module = setup.engine();
    let ready: (info: EngineInfo) => void = () => undefined;
    let lost: (failure: EngineFailure) => void = () => undefined;
    this.#info = new Promise<EngineInfo>((resolve, reject) => {
      ready = resolve;
      lost = reject;
    });
    this.#info.catch(() => undefined);
    this.#infoLost = (failure) => {
      lost(failure);
    };
    this.#spine = new CrewMember('spine', this.#spawn, this.#module, {
      ready: (info) => {
        ready(info);
      },
      replay: () => null,
      lost: (error) => {
        lost(new EngineFailure(error));
      },
    });
    this.#spare = this.#filmMember();
  }

  /** The engine's info, from the spine worker's `ready`; rejected with `pass-cancelled` if the crew is stopped first. */
  info(): Promise<EngineInfo> {
    return this.#info;
  }

  /**
   * A new call over `films`. It is stopped with `pass-cancelled` when one
   * of them is replaced or retired, or when `signal` aborts.
   */
  scope(films: Iterable<FilmKind>, signal?: AbortSignal): CallScope {
    const aborted = () => {
      scope.stop(passCancelled('aborted by its signal'));
    };
    const scope: CallScope = new CallScope(films, () => {
      this.#scopes.delete(scope);
      signal?.removeEventListener('abort', aborted);
    });
    if (this.#stopped) {
      scope.stop(passCancelled('the engine was stopped'));
      return scope;
    }
    this.#scopes.add(scope);
    if (signal?.aborted === true) scope.stop(passCancelled('aborted by its signal'));
    else signal?.addEventListener('abort', aborted, { once: true });
    return scope;
  }

  /** Runs one job for a call: on the spine worker, or on `place.film`'s worker. */
  run<J extends JobName>(job: J, input: JobInput<J>, place: JobPlace): Promise<JobOutput<J>> {
    return new Promise<JobOutput<J>>((resolve, reject) => {
      const { scope, film } = place;
      if (scope.stopped !== null) {
        reject(new EngineFailure(scope.stopped));
        return;
      }
      const member = JOBS[job].worker === 'spine' ? this.#spine : film === undefined ? undefined : this.#films.get(film);
      if (member === undefined) {
        reject(new EngineFailure({ code: 'engine-out-of-sync', message: 'no film of this kind is open', detail: film ?? null }));
        return;
      }
      let unlisten: () => void = () => undefined;
      const ticket: Ticket = {
        id: this.#nextId++,
        job,
        input,
        replay: false,
        progress: (stage, fraction) => {
          if (!scope.settled) place.progress?.(stage, fraction);
        },
        frame: (frame) => {
          if (!scope.settled) place.frame?.(frame);
        },
        done: (output) => {
          unlisten();
          if (job === 'open-film' && film !== undefined) {
            const opened = input as JobInput<'open-film'>;
            this.#bytes.keep(film, { bytes: opened.bytes, saved: opened.saved });
          }
          resolve(output as JobOutput<J>);
        },
        failed: (error) => {
          unlisten();
          reject(new EngineFailure(error));
        },
      };
      unlisten = scope.onStop((error) => {
        member.withdraw(ticket, job === 'open-film');
        reject(new EngineFailure(error));
      });
      member.add(ticket);
    });
  }

  /**
   * Reads a film into its worker: the spare for the first film, a worker
   * of its own for each later one, the same worker when it replaces an
   * open film of its kind.
   */
  openFilm(kind: FilmKind, bytes: ArrayBuffer, saved: Film | null): Promise<OpenedFilm> {
    this.#cutOff(kind, passCancelled(`the ${kind} film was replaced`));
    if (!this.#stopped && !this.#films.has(kind)) {
      this.#films.set(kind, this.#spare ?? this.#filmMember());
      this.#spare = null;
    }
    const scope = this.scope([kind]);
    return scope.settle(() => this.run('open-film', { kind, bytes, saved }, { scope, film: kind }));
  }

  /** Forgets a film: its calls are stopped, its worker closed and its bytes dropped. */
  retireFilm(kind: FilmKind): void {
    this.#cutOff(kind, passCancelled(`the ${kind} film was retired`));
    this.#films.get(kind)?.close();
    this.#films.delete(kind);
    this.#bytes.drop(kind);
  }

  /** Stops every call and closes every worker. */
  stop(): void {
    this.#stopped = true;
    this.#infoLost(new EngineFailure(passCancelled('the engine was stopped before it was ready')));
    for (const scope of [...this.#scopes]) scope.stop(passCancelled('the engine was stopped'));
    this.#spine.close();
    this.#spare?.close();
    this.#spare = null;
    for (const member of this.#films.values()) member.close();
    this.#films.clear();
  }

  #cutOff(kind: FilmKind, error: ReturnType<typeof passCancelled>): void {
    for (const scope of [...this.#scopes]) {
      if (scope.films.has(kind)) scope.stop(error);
    }
  }

  #filmMember(): CrewMember {
    const member: CrewMember = new CrewMember('film', this.#spawn, this.#module, {
      ready: () => undefined,
      lost: () => undefined,
      replay: (crashedReadingAgain) => {
        const kind = this.#kindOf(member);
        if (kind === null) return null;
        if (crashedReadingAgain) {
          this.#bytes.drop(kind);
          return null;
        }
        const kept = this.#bytes.get(kind);
        if (kept === undefined) return null;
        return {
          id: this.#nextId++,
          job: 'open-film',
          input: { kind, bytes: kept.bytes, saved: kept.saved },
          replay: true,
          progress: () => undefined,
          frame: () => undefined,
          done: () => undefined,
          failed: () => {
            if (this.#bytes.get(kind) === kept) this.#bytes.drop(kind);
          },
        };
      },
    });
    return member;
  }

  #kindOf(member: CrewMember): FilmKind | null {
    for (const [kind, held] of this.#films) if (held === member) return kind;
    return null;
  }
}
