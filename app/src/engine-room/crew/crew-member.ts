// One engine worker and the jobs waiting for it.
//
// It sends one job at a time and the next only once the last has
// answered. Messages about a job that is no longer running, or from a
// worker that has been replaced, are dropped.
//
// When its worker crashes (a panic, an error event, or a slice of a pass
// running past two seconds), the job it was running fails with the crash,
// a fresh worker starts from the compiled module already in hand, and a
// film worker reads its film again from the bytes the crew kept before
// any waiting job runs. A worker that fails while starting is not
// replaced: starting again would fail the same way, so every job sent to
// it fails with that error instead.

import type { EngineError, EngineInfo, Stage, WatchFrame } from '@/shapes';
import { JOBS, type JobName } from '../job-table';
import type { FromWorker, ToWorker, WorkerRole } from '../messages';
import { engineErrorOf, passCancelled } from './call-scope';
import { JobQueue } from './job-queue';

/** The parts of a Worker the crew uses. */
export interface WorkerLike {
  postMessage(message: ToWorker): void;
  terminate(): void;
}

/** What a worker tells the crew: its messages, and that it failed outright. */
export interface WorkerEvents {
  message(message: FromWorker): void;
  failed(detail: string): void;
}

/** Starts a worker for `role`, wired to `events`. */
export type SpawnWorker = (role: WorkerRole, events: WorkerEvents) => WorkerLike;

/** One job for one worker, and where what it says goes. */
export interface Ticket {
  readonly id: number;
  readonly job: JobName;
  readonly input: unknown;
  /** The `open-film` a fresh worker runs to read its film again; no call waits on it. */
  readonly replay: boolean;
  progress(stage: Stage, fraction: number): void;
  frame(frame: WatchFrame): void;
  done(output: unknown): void;
  failed(error: EngineError): void;
}

/** A pass's worker is replaced when this long goes by with no word about it. */
export const STUCK_PASS_MS = 2000;

export interface MemberHooks {
  /** The worker's first `ready`. */
  ready(info: EngineInfo): void;
  /**
   * The film a fresh worker must read again, as a ticket to run before
   * anything else, or null. `crashedReadingAgain` is true when the worker
   * crashed while reading it again, so it should not be tried once more.
   */
  replay(crashedReadingAgain: boolean): Ticket | null;
  /** The worker could not start, and every job sent to it fails with `error`. */
  lost(error: EngineError): void;
}

export class CrewMember {
  readonly role: WorkerRole;
  readonly #spawn: SpawnWorker;
  readonly #module: Promise<WebAssembly.Module>;
  readonly #hooks: MemberHooks;
  readonly #queue = new JobQueue<Ticket>();
  #worker: WorkerLike | null = null;
  #generation = 0;
  #state: 'starting' | 'ready' | 'lost' | 'closed' = 'starting';
  #everReady = false;
  #lost: EngineError | null = null;
  #running: { ticket: Ticket; withdrawn: boolean } | null = null;
  #watchdog: ReturnType<typeof setTimeout> | null = null;

  constructor(role: WorkerRole, spawn: SpawnWorker, module: Promise<WebAssembly.Module>, hooks: MemberHooks) {
    this.role = role;
    this.#spawn = spawn;
    this.#module = module;
    this.#hooks = hooks;
    this.#start();
  }

  /** Queues `ticket`. A replaceable job's older waiting twin fails with `pass-cancelled`. */
  add(ticket: Ticket): void {
    if (this.#state === 'lost' || this.#state === 'closed') {
      ticket.failed(this.#lost ?? passCancelled('the worker was closed'));
      return;
    }
    this.#queue.add(ticket)?.failed(passCancelled('replaced by a newer call'));
    this.#pump();
  }

  /**
   * Takes back a job whose call was stopped. A waiting one leaves the
   * queue; a running pass is told to cancel and anything more it says is
   * dropped. A running job that changes what the worker holds (opening a
   * film) cannot be left to finish, so with `restart` the worker is
   * replaced and reads its open film again.
   */
  withdraw(ticket: Ticket, restart: boolean): void {
    if (this.#queue.remove(ticket)) return;
    const running = this.#running;
    if (running?.ticket !== ticket) return;
    running.withdrawn = true;
    if (restart) {
      this.#respawn(false);
      return;
    }
    if (JOBS[ticket.job].stepped) this.#send({ kind: 'cancel', id: ticket.id });
  }

  /** Closes the worker for good. Anything still waiting fails with `pass-cancelled`. */
  close(): void {
    if (this.#state === 'closed') return;
    this.#state = 'closed';
    this.#stopWatchdog();
    this.#worker?.terminate();
    this.#worker = null;
    this.#generation += 1;
    const running = this.#running;
    this.#running = null;
    if (running !== null && !running.withdrawn) running.ticket.failed(passCancelled('the worker was closed'));
    for (const ticket of this.#queue.clear()) ticket.failed(passCancelled('the worker was closed'));
  }

  #start(): void {
    this.#generation += 1;
    const generation = this.#generation;
    this.#state = 'starting';
    const current = () => generation === this.#generation;
    this.#worker = this.#spawn(this.role, {
      message: (message) => {
        if (current()) this.#onMessage(message);
      },
      failed: (detail) => {
        if (current()) this.#onWorkerFailed(detail);
      },
    });
    this.#module.then(
      (module) => {
        if (!current() || this.#state !== 'starting') return;
        const unsent = this.#send({ kind: 'boot', module });
        if (unsent !== null) this.#lose({ code: 'engine-load-failed', message: 'the engine could not be handed to a worker', detail: unsent });
      },
      (thrown: unknown) => {
        if (current()) this.#lose(engineErrorOf(thrown));
      },
    );
  }

  #onMessage(message: FromWorker): void {
    switch (message.kind) {
      case 'ready':
        if (this.#state !== 'starting') return;
        this.#state = 'ready';
        if (!this.#everReady) {
          this.#everReady = true;
          this.#hooks.ready(message.info);
        }
        this.#pump();
        return;
      case 'crashed':
        this.#crash(message.error);
        return;
      case 'progress':
      case 'frame':
      case 'done':
      case 'failed': {
        const running = this.#running;
        if (running?.ticket.id !== message.id) return;
        if (message.kind === 'progress' || message.kind === 'frame') {
          if (JOBS[running.ticket.job].stepped) this.#startWatchdog(running.ticket.job);
          if (running.withdrawn) return;
          if (message.kind === 'progress') running.ticket.progress(message.stage, message.fraction);
          else running.ticket.frame(message.frame);
          return;
        }
        this.#stopWatchdog();
        this.#running = null;
        if (!running.withdrawn) {
          if (message.kind === 'done') running.ticket.done(message.output);
          else running.ticket.failed(message.error);
        }
        this.#pump();
        return;
      }
    }
  }

  #onWorkerFailed(detail: string): void {
    const starting = this.#state === 'starting';
    this.#crash(
      starting
        ? { code: 'engine-load-failed', message: 'a worker did not start', detail }
        : { code: 'engine-crashed', message: 'a worker stopped', detail },
    );
  }

  #crash(error: EngineError): void {
    if (this.#state === 'lost' || this.#state === 'closed') return;
    if (this.#state === 'starting') {
      this.#lose(error);
      return;
    }
    const running = this.#running;
    this.#running = null;
    if (running !== null && !running.withdrawn) running.ticket.failed(error);
    this.#respawn(running?.ticket.replay === true);
  }

  /** A fresh worker in place of this one, which reads the open film again first. */
  #respawn(crashedReadingAgain: boolean): void {
    this.#stopWatchdog();
    this.#worker?.terminate();
    this.#running = null;
    this.#start();
    const replay = this.#hooks.replay(crashedReadingAgain);
    if (replay !== null) this.#queue.addFirst(replay);
  }

  #lose(error: EngineError): void {
    this.#state = 'lost';
    this.#lost = error;
    this.#stopWatchdog();
    this.#worker?.terminate();
    this.#worker = null;
    const running = this.#running;
    this.#running = null;
    if (running !== null && !running.withdrawn) running.ticket.failed(error);
    for (const ticket of this.#queue.clear()) ticket.failed(error);
    this.#hooks.lost(error);
  }

  #pump(): void {
    while (this.#state === 'ready' && this.#running === null) {
      const ticket = this.#queue.take();
      if (ticket === undefined) return;
      this.#running = { ticket, withdrawn: false };
      const sent = this.#send({ kind: 'run', id: ticket.id, job: ticket.job, input: ticket.input });
      if (sent !== null) {
        this.#running = null;
        ticket.failed({ code: 'engine-out-of-sync', message: 'the job could not be sent to its worker', detail: sent });
        continue;
      }
      if (JOBS[ticket.job].stepped) this.#startWatchdog(ticket.job);
    }
  }

  /** Posts `message`; returns why it could not be sent, or null. */
  #send(message: ToWorker): string | null {
    try {
      this.#worker?.postMessage(message);
      return null;
    } catch (thrown) {
      return String(thrown);
    }
  }

  #startWatchdog(job: JobName): void {
    this.#stopWatchdog();
    this.#watchdog = setTimeout(() => {
      this.#watchdog = null;
      this.#crash({ code: 'engine-crashed', message: 'a slice of a pass ran past two seconds', detail: job });
    }, STUCK_PASS_MS);
  }

  #stopWatchdog(): void {
    if (this.#watchdog !== null) clearTimeout(this.#watchdog);
    this.#watchdog = null;
  }
}
