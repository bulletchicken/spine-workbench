# Worker messages

The protocol between the main thread and every engine worker. Frozen: it
changes only through a contract-change pull request. The source is
`app/src/engine-room/messages.ts`.

## The workers

- Plain module workers. No WebAssembly threads, no `SharedArrayBuffer`,
  no cross-origin isolation.
- **Film workers:** one per loaded film, up to four. Each holds that
  film's `FilmDesk`.
- **Spine worker:** exactly one. It runs the chain pass (which needs every
  film), the case checks, packing and unpacking case files, made-up spines
  and answer-key comparisons.
- Workers never talk to each other. The main thread's auto-pass conductor
  runs film passes in parallel, hands their small results to the spine
  worker's chain pass, then asks each film worker to place points.
- Every worker instantiates the same compiled module. Workers never fetch:
  the main thread compiles the engine once and posts the module.

## Messages

```ts
type ToWorker =
  | { kind: 'boot'; module: WebAssembly.Module }
  | { kind: 'run'; id: number; job: JobName; input: unknown }
  | { kind: 'cancel'; id: number };

type FromWorker =
  | { kind: 'ready'; info: EngineInfo }
  | { kind: 'progress'; id: number; stage: Stage; fraction: number }
  | { kind: 'frame'; id: number; frame: WatchFrame }
  | { kind: 'done'; id: number; output: unknown }
  | { kind: 'failed'; id: number; error: EngineError }
  | { kind: 'crashed'; error: EngineError };
```

```
main thread                         worker
boot { module }          ───────▶   initSync({ module }), engine_info()
                         ◀───────   ready { info }   or   crashed { error }
run { id, job, input }   ───────▶
                         ◀───────   progress, frame …  (stepped jobs only)
                         ◀───────   done { output }    or   failed { error }
cancel { id }            ───────▶   (stepped jobs stop between slices)
```

## Rules

1. **Order.** For each job, messages arrive in order, and `done` or
   `failed` is always the last one with that id.
2. **Fingerprint handshake.** A worker answers `ready` only when the
   engine's `shapesFingerprint` equals the app's generated
   `SHAPES_FINGERPRINT`. Otherwise it posts `crashed` with
   `engine-out-of-sync` and closes, so a stale engine never touches data.
3. **Unknown jobs** are answered `failed` with `engine-out-of-sync`.
4. **Typed payloads.** `input` and `output` are typed per job by
   `app/src/engine-room/job-table.ts` (see the [engine API](engine-api.md)),
   so a wrong payload is a compile error rather than a refusal.
5. **One job at a time** per worker. Jobs the table marks `replaceable`
   (hover previews, live checks) keep at most one waiting; a newer one
   replaces it.
6. **Transfers.** `output` and frame payloads go in the transfer list;
   nothing is copied twice.
7. **Stepped jobs** (`film-pass`, `chain-pass`) run in slices of about
   8 ms, timed in the worker with `performance.now()`; the engine itself
   has no clock. Progress is sent at most ten times a second; sweep frames
   at most thirty times a second, newest first, and keyframes are never
   dropped (see [watch frames](watch-frames.md)).
8. **Cancel** is a flag checked between slices. The pass is freed and the
   job ends `failed` with `pass-cancelled`. A slice that runs past two
   seconds gets the worker terminated and replaced.
9. **Crashes.** A Rust panic ends in a WebAssembly trap. The panic hook
   has already stored the message; the worker reads `last_panic()`, posts
   `crashed` with `engine-crashed` and the message as the detail, and
   closes. The crew starts a fresh worker from the cached module and
   reopens the film from the bytes it kept for the session. Marks live on
   the main thread, so nothing is lost.

## Errors

Every failure is an `EngineError { code, message, detail }`. `code` is
one value of the generated `ErrorCode` union; `message` is English for
developers; `detail` says where or what (a JSON path, a codec, a reason)
or is `null`. Users never see `message`: `app/src/words/faults.ts` words
every code, in an exhaustive switch, so a new code does not compile until
it is worded. `EngineClient` promises reject only with `EngineFailure`,
which carries the `EngineError`.
