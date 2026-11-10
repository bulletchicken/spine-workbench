// The worker message protocol. FROZEN: changes only through a
// contract-change pull request.
//
// Every engine worker speaks it, film worker or spine worker alike:
//
//   main thread                         worker
//   boot { module }          ───────▶   initSync({ module }), engine_info()
//                            ◀───────   ready { info }   or   crashed { error }
//   run { id, job, input }   ───────▶
//                            ◀───────   progress, frame … (stepped jobs only)
//                            ◀───────   done { output }  or   failed { error }
//   cancel { id }            ───────▶   (stepped jobs stop between slices)
//
// Rules:
// - For each job, messages arrive in order, and `done` or `failed` is
//   always the last one with that id.
// - The worker never fetches: the main thread compiles the engine once and
//   posts the compiled module in `boot`.
// - A worker answers `ready` only when the engine's shapes fingerprint
//   equals the app's. Otherwise it posts `crashed` with
//   `engine-out-of-sync` and closes, so a stale engine never touches data.
// - A job the worker does not know is answered `failed` with
//   `engine-out-of-sync`.
// - A panic posts `crashed` with `engine-crashed` (the panic message as the
//   detail), and the worker closes. Its module is gone; the crew starts a
//   fresh worker from the cached module.
// - `output` and frame payloads are sent in the transfer list, never
//   copied twice.

import type { EngineError, EngineInfo, Stage, WatchFrame } from '@/shapes';
import type { JobName } from './job-table';

/** From the main thread to a worker. */
export type ToWorker =
  | { kind: 'boot'; module: WebAssembly.Module }
  | { kind: 'run'; id: number; job: JobName; input: unknown }
  | { kind: 'cancel'; id: number };

/** From a worker to the main thread. */
export type FromWorker =
  | { kind: 'ready'; info: EngineInfo }
  | { kind: 'progress'; id: number; stage: Stage; fraction: number }
  | { kind: 'frame'; id: number; frame: WatchFrame }
  | { kind: 'done'; id: number; output: unknown }
  | { kind: 'failed'; id: number; error: EngineError }
  | { kind: 'crashed'; error: EngineError };

/** Which engine worker a job runs in. */
export type WorkerRole = 'film' | 'spine';
