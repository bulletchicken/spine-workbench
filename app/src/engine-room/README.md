# engine-room

Everything between the screens and the Rust engine. Screens import
`engine-client.ts` and nothing further down.

```
screen
  │  engine.regionWand('front', input)                       engine-client.ts (frozen)
  ▼
crew ── picks the front film's worker, queues the job        crew/
  │  postMessage { kind: 'run', id, job: 'region-wand', input }   messages.ts (frozen)
  ▼
film worker ── finds the handler for 'region-wand'           workers/boot.ts, workers/film-worker/jobs/
  │  desk.region_wand(input)                                  job-table.ts (frozen) types input and output
  ▼
wasm-bridge ── converts the shapes, calls engine/toolbox     engine/wasm-bridge/src/exports/toolbox.rs
  │
  ▲  RegionWandOutput, or a thrown EngineError
film worker
  │  postMessage { kind: 'done', id, output }  (or 'failed'; a panic posts 'crashed')
  ▼
crew ── resolves the promise, or rejects it with an EngineFailure
  ▼
screen ── applies it as an edit basedOn its case version; the case store drops it if stale
```

Before any job, `load-engine.ts` fetches and compiles the engine's `.wasm`
once, from the page's own origin. It is the only file in the app allowed
to make a network call. Each worker receives the compiled module in its
`boot` message, starts it with `initSync`, and answers `ready` only if the
engine was built from the same shapes as the app.

| File | Holds |
|---|---|
| `engine-client.ts` | the `EngineClient` interface screens use, and `EngineFailure` |
| `job-table.ts` | every job: which worker runs it, its input and output types |
| `messages.ts` | the messages between the main thread and a worker |
| `load-engine.ts` | the one sanctioned fetch: our own `.wasm`, compiled once |
| `workers/boot.ts` | how a worker starts the engine and reports `ready` |
| `workers/film-worker/film.worker.ts` | a film worker: one per film |
| `workers/spine-worker/spine.worker.ts` | the spine worker: exactly one |
| `wasm-build/` | written by `cargo chores wasm`; never committed |
