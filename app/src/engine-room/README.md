# engine-room

Everything between the screens and the Rust engine. Screens import
`engine-client.ts` (the `EngineClient` type) and `use-engine.tsx` (to get
one), and nothing further down.

```
screen
  │  useEngine().regionWand('front', input)                  engine-client.ts (frozen), use-engine.tsx
  ▼
real engine ── one call, a scope over the films it is about   crew/real-engine.ts, crew/call-scope.ts
  ▼
crew ── picks the front film's worker, queues the job        crew/crew.ts, crew/crew-member.ts
  │  postMessage { kind: 'run', id, job: 'region-wand', input }   messages.ts (frozen)
  ▼
film worker ── runs the file for 'region-wand'               workers/boot.ts, workers/film-worker/jobs/
  │  room.desk().region_wand(input)                           job-table.ts (frozen) types input and output
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
engine was built from the same shapes as the app and every job file
matches the job table.

## Files

| File | Holds |
|---|---|
| `engine-client.ts` | the `EngineClient` interface screens use, and `EngineFailure` |
| `job-table.ts` | every job: which worker runs it, its input and output types |
| `messages.ts` | the messages between the main thread and a worker |
| `load-engine.ts` | the one sanctioned fetch: our own `.wasm`, compiled once |
| `pick-engine.ts` | which engine a page gets: always the real one in production |
| `use-engine.tsx` | `EngineProvider` and `useEngine()` |
| `payloads.ts` | float and validity payloads as views; transfer lists |
| `crew/crew.ts` | the spine worker, a spare film worker, one worker per film; calls cut off when their film is replaced or retired |
| `crew/crew-member.ts` | one worker: its queue, one job at a time, crash and stuck-pass recovery |
| `crew/job-queue.ts` | a worker's waiting jobs, at most one of each replaceable kind |
| `crew/call-scope.ts` | one call from start to answer, and every way it can be stopped |
| `crew/film-bytes.ts` | each open film's bytes, for a fresh worker to read again |
| `crew/real-engine.ts` | `EngineClient` over the crew |
| `crew/auto-pass-conductor.ts` | film passes side by side, the chain pass, then points film by film; with the trained finder, its pass on the front film beside them, and the chain pass and points for the other films only |
| `crew/spawn-worker.ts`, `crew/start-real-engine.ts` | real workers, and the real engine started |
| `crew/test-rig.ts` | fake workers for the crew's tests, running the real job loop |
| `workers/boot.ts` | how a worker starts, finds its job files and runs jobs |
| `workers/step-loop.ts` | a stepped pass in ~8 ms slices: progress, frame thinning, cancel |
| `workers/film-worker/film.worker.ts` | a film worker: one per film |
| `workers/spine-worker/spine.worker.ts` | the spine worker: exactly one |
| `workers/<film\|spine>-worker/jobs/` | one file per job, written by the slice that builds the job's engine call |
| `stand-in/` | the stand-in engine: development and tests only, never shipped |
| `wasm-build/` | written by `cargo chores wasm`; never committed |

## Adding a job

A job is one file and nothing else: no list to edit, no registration.

1. Name the file after the job, exactly as `job-table.ts` spells it, in
   the folder of the worker the table says runs it:
   `workers/film-worker/jobs/<job>.ts` or `workers/spine-worker/jobs/<job>.ts`.
2. Default-export `defineJob('<job>', handler)`. A plain job gives
   `run(input, room)` and returns the job's output. A stepped job
   (`film-pass`, `chain-pass`, `trained-pass`) gives `start(input,
   room)` and returns a pass with `advance(workUnits)`, `finish()` and
   `free()`. The types of
   `input` and the answer come from the job table, so a wrong one does
   not compile. `finish()` must consume the engine's pass (in Rust,
   `fn finish(self)`), because the step loop calls `free()` only when a
   pass is stopped or fails, never after `finish()`.
3. A film worker's jobs reach the film through `room.desk()`. Only
   `open-film` calls `room.holdDesk(desk)`, and only once the new desk
   has opened, so a film that cannot be read leaves the open one in
   place (the old desk is freed when the new one is held).
4. Let the engine's errors through: a thrown `EngineError` (or
   `EngineFailure`) fails only that job, and a WebAssembly trap crashes
   the worker, which the crew replaces. Return fresh typed arrays, never
   views into wasm memory: every buffer in an answer or a frame is handed
   over in the transfer list.
5. A stepped job's `start` must return quickly: the main thread replaces
   a worker that says nothing about its pass for two seconds, and the
   time `start` takes counts.

The worker finds the file with `import.meta.glob`, and checks it at
start-up: a file named after no job, in the wrong worker's folder, or
giving `run` for a stepped job (or `start` for a plain one) makes the
worker refuse to start with `engine-out-of-sync`. The test "finds nothing
wrong with the job files in both workers" in `workers/boot.test.ts`
fails the same way, before anything ships. Test files (`*.test.ts`) in
`jobs/` are left out of the glob.

A worked example, `workers/film-worker/jobs/sense-geometry.ts`:

```ts
// The engine's guesses about one film: scale, upright, facing, markers.

import type { GeometryGuess } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('sense-geometry', {
  run: (_input, room) => room.desk().sense_geometry() as GeometryGuess,
});
```

And a stepped one, `workers/film-worker/jobs/film-pass.ts`, adapting the
engine's `FilmPass` to the pass the step loop runs:

```ts
import type { FilmPassResult, Step } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('film-pass', {
  start: ({ film }, room) => {
    const pass = room.desk().start_film_pass(film);
    return {
      advance: (workUnits) => pass.advance(workUnits) as Step,
      finish: () => pass.finish() as FilmPassResult,
      free: () => {
        pass.free();
      },
    };
  },
});
```

The stand-in answers from its story only the jobs it has a handler for
(`stand-in/film-jobs.ts`, `stand-in/spine-jobs.ts`); any other job fails
there with `engine-out-of-sync`, so a job the screens call needs one.

## The crew's rules

- **Start-up.** The spine worker and one spare film worker boot at once,
  so the first film opened takes a worker that is already running. Each
  later film starts its own worker when it is opened.
- **One job at a time** per worker, in the order asked. A replaceable
  job (`tool-preview`, `check-case`, `compare-with-answer-key`) keeps at
  most one waiting: the older waiting one rejects with `pass-cancelled`
  and the newer one joins the back of the queue. A running one is never
  replaced.
- **Replacing a film.** `openFilm` for a kind already open first stops
  every call still waiting or running on the old film with
  `pass-cancelled`, a `runAutoPass` or `saveCase` that includes it as a
  whole, then queues the new `open-film` on the same worker, so a call
  made afterwards goes to the new film. If the new film cannot be read,
  the worker keeps the old one, and so do the kept bytes. A film still
  opening when it is replaced cannot be left to finish (it would change
  what the worker holds), so its worker is replaced and reads the film
  that was open again first. `retireFilm` stops its calls the same way
  and closes the worker.
- **Order.** Each job's frames reach `onFrame` in the order the engine
  made them. Nothing reaches a call's callbacks once it is stopped.
- **Crashes.** A panic, a worker error event, or a pass silent for two
  seconds fails the running job with `engine-crashed` (unless its call
  was already stopped), and a fresh worker starts from the compiled
  module in hand. A film worker reads its film again from the kept bytes
  before any waiting job; if reading it again crashes too, the film is
  dropped rather than tried a third time. A worker that fails while
  starting (a stale engine, an engine that will not load) is not
  replaced, and every call sent to it fails with that error.
- **Calls on a film that is not open** reject with `engine-out-of-sync`.

## The stand-in

`stand-in/stand-in-engine.ts` starts the same crew with in-page workers
(`stand-in/stand-in-worker.ts`) that speak the worker protocol and answer
from a `StandInStory` (`sample-case/story-shape.ts`). So every rule above
holds for it by construction, and `crew/cut-offs.test.ts` holds both
engines to the same tests. `pick-engine.ts` hands it the sample case's
story once `sample-case/story.ts` exists, and its own small story
(`stand-in/small-story.ts`) until then.

How it answers:

| Call | Stand-in answer |
|---|---|
| `openFilm` | the story film whose file has exactly these bytes, after a moment, or the film one of the made-up spine's PNGs shows, with the made-up spine's facts (`madeUp.films`); anything else is `film-unknown-format` |
| `displayPixels`, `senseGeometry` | the pixels of the story film the bytes show; its guess, or the made-up spine's guess for a film opened from its PNG (for a film of another view, the slot's own story film's guess: see below) |
| `scaleFromRuler`, `scaleFromKnownObject` | the length in mm over the length in pixels; too short is `scale-too-short`, no length `scale-bad-length` |
| `runAutoPass` | each film's story frames, then the chain's, through the real step loop; the story's marks with ids counting up from the first free id, film by film. The same whatever `frontFinder` says: with `trained`, the front film's trained pass plays its story frames from every stage and answers its story marks, and the chain pass runs on the other films, so only the front-and-side check is missing |
| `previewTool`, `regionWand`, `boundaryWand`, `resolvePins` | plain geometry over the marks given (`stand-in/tool-preview.ts`, `region-wand.ts`, `boundary-wand.ts`, `pins.ts`) |
| `checkCase` | worked out from the case given, by the sample case's `findingsAt` rules and a few more (`stand-in/checks.ts`) |
| `saveCase`, `openCase` | a case file of its own (JSON, not the engine's zip) that reads back exactly, each film stored as the made-up spine's PNG of it; `openCase` opens no film, and refuses the story's refused case with its error |
| `makeUpSpine`, `compareWithAnswerKey` | the story's made-up spine (`madeUp.spine`); plain distances to its answer key |

A film's pixels, thumbnail and saved PNG are always those of the film its
bytes show. Its guess, its pass's frames and its marks follow that story
film while its marks suit the slot it was opened into: the same film, or a
bending film in either bending slot. A film of another view (the side
film's file in the front slot, the front film's in a bending slot) answers
as the slot's own story film, so every answer is one the slot can hold, as
the engine's are. A stand-in film pass's strip names the story film it
answered as, so the chain pass follows it too. A pass sends one story frame
at a time and spends the gap before the next, and before its answer, in
empty slices at most `HEARTBEAT_MS` (400 ms) apart, so it takes its
scripted time and the crew never takes a slow script for a stuck pass.

How long each answer takes, the pass's pace and the tools' numbers are in
six scripts, one per step and flow: `stand-in/scripts/films.ts`,
`size-and-facing.ts`, `find.ts`, `adjust.ts`, `save.ts` and `reopen.ts`.
The step slices own them from here on. In development, `?slow` runs the
pass at the slow pace, and `?engine=real` asks for the real engine.
