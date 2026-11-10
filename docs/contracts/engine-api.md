# Engine API

Every way the app reaches the engine, from the screens down to Rust.
Frozen: each layer changes only through a contract-change pull request.

```
screens ── EngineClient ── crew of workers ── wasm-bridge ── engine crates
           (TypeScript)    (job table)        (wasm-bindgen)  (plain Rust)
```

No layer skips one: screens never touch workers, and workers never hold
maths. All points are in original film pixels
([pixel space](../conventions/pixel-space.md)); every shape is generated
from `engine/shapes` and imported in TypeScript from `@/shapes`.

## EngineClient

`app/src/engine-room/engine-client.ts` is the only engine import screens
use. The real engine (the crew of workers) and the stand-in engine (dev
and tests only, never shipped) both implement it.

| Call | Returns | Job(s) |
|---|---|---|
| `info()` | `EngineInfo` | the `ready` message |
| `openFilm(kind, bytes, saved)` | `{ facts, thumbnail }` | `open-film` |
| `displayPixels(kind)` | `Uint16Array` (half-float bits) | `display-pixels` |
| `retireFilm(kind)` | nothing; the film's worker closes | |
| `senseGeometry(kind)` | `GeometryGuess` | `sense-geometry` |
| `scaleFromRuler(kind, a, b, mm)` | `ScaleResult` | `scale-from-ruler` |
| `scaleFromKnownObject(kind, near, edge, diameterMm)` | `ScaleResult` | `scale-from-known-object` |
| `runAutoPass(input, run?)` | `AutoPassResult` | `film-pass` on every film, `chain-pass`, `place-points` on every film |
| `previewTool(kind, input)` | `ToolPreview` | `tool-preview` |
| `regionWand(kind, input)` | `RegionWandOutput` | `region-wand` |
| `boundaryWand(kind, input)` | `BoundaryWandOutput` | `boundary-wand` |
| `resolvePins(kind, input)` | `PinOutput` | `resolve-pins` |
| `checkCase(case)` | `CheckFinding[]` | `check-case` |
| `saveCase(case)` | `.spinecase` bytes | `encode-png` on every film, then `pack-case` |
| `openCase(bytes)` | `OpenedCase` | `unpack-case`; it opens no film |
| `makeUpSpine(recipe)` | `MadeUpSpine` | `make-up-spine` |
| `compareWithAnswerKey(marks, key)` | `AnswerKeyComparison` | `compare-with-answer-key` |

`openCase` reads and checks the file and opens no film. The screen then
opens each film through `openFilm` with its saved record. `openFilm` for
a kind that is already open replaces that film.

When a film is replaced or retired, every call on it that is still
waiting or running rejects with `pass-cancelled`, the code a replaced
call rejects with: an `openFilm` still opening it, `displayPixels`,
`senseGeometry`, the two scale calls, `previewTool`, `regionWand`,
`boundaryWand` and `resolvePins`. A `runAutoPass` or `saveCase` that
includes the film and has not settled rejects as a whole with
`pass-cancelled`, because its answer would describe a film that is no
longer the one open: the pass's other films are cancelled with it and
nothing is applied. Frames already delivered stay delivered, and none
arrives after the rejection. Calls on other films are untouched, and a
call made after the replacing `openFilm` goes to the new film. If the
replacing `openFilm` fails, the film that was open stays open and
unchanged, so a file that cannot be read costs nothing but the calls it
cut off, which the screen may make again.

Only a `runAutoPass` rejected this way is a stopped search. Any other
call rejected this way is not, and the screen does not word it as one:
it lets a call on the old film go by, and the save sheet starts the
save again from the case as it now stands.

`run` (on `runAutoPass` only) takes `onProgress`, `onFrame` and an
`AbortSignal`; aborting rejects with `pass-cancelled`. Every promise
rejects only with `EngineFailure`. The engine keeps nothing but each
film's pixels and caches: every edit request, and the auto pass, carries
the case store's version it was computed from (not `CASE_VERSION`, the
file format's), and every answer echoes that version, so the case store
can drop an answer that is stale (the rules in
`app/src/case-state/case-edits.ts`).

### From an auto pass to edits

`AutoPassInput.caseVersion` is the version the films were taken from.
The client stamps it on `AutoPassResult.caseVersion` itself: the jobs
between workers (`film-pass`, `chain-pass`, `place-points`) never reach
the case store, so they carry no version and no answer reaches it
without one. The screens then apply the result as one action: one
`replace-marks` edit per entry of `AutoPassResult.marks`, in order:

```ts
apply(result.marks.map((marks) => ({ kind: 'replace-marks', marks, cause: { by: 'auto-pass' }, basedOn: result.caseVersion })));
```

The case store judges staleness per film: each edit is dropped if its
own film changed after `basedOn` (any applied edit on it, undo and redo
included, or an `open-case`), so the first film's edit landing does not
make the others stale. It is also dropped if it uses a vertebra id the
case may have given out since `basedOn` (an id its film does not hold
and that is below the case's `nextVertebraId`), so one id never ends up
on two vertebrae. The edits go in the order of `marks` because each film's
new ids lie above the previous film's. The kept edits are one action,
one undo step; when none is kept, nothing changes and no undo step is
added.

Applying the action reports which edits were kept. When only some are
kept, the screens show `twoFilms` only if every film's edit was kept,
and each warning only if its film's edit was kept, so nothing on screen
describes marks the case does not hold.

## Jobs

`app/src/engine-room/job-table.ts` maps each job to its worker, its input
and output types, and whether it is stepped or replaceable. A film worker
holds one film, so its jobs never say which.

| Job | Worker | Input | Output | Stepped | Replaceable |
|---|---|---|---|---|---|
| `open-film` | film | `{ kind, bytes, saved: Film \| null }` | `{ facts, thumbnail }` | | |
| `display-pixels` | film | `null` | `Uint16Array` | | |
| `encode-png` | film | `{ film: Film }` | `Uint8Array` (PNG) | | |
| `sense-geometry` | film | `null` | `GeometryGuess` | | |
| `scale-from-ruler` | film | `{ a, b, mm }` | `ScaleResult` | | |
| `scale-from-known-object` | film | `{ near, edge, diameterMm }` | `ScaleResult` | | |
| `film-pass` | film | `{ film: Film }` | `FilmPassResult` | yes | |
| `place-points` | film | `{ chain: FilmChain, firstVertebraId }` | `FilmMarks` | | |
| `tool-preview` | film | `ToolPreviewInput` | `ToolPreview` | | yes |
| `region-wand` | film | `RegionWandInput` | `RegionWandOutput` | | |
| `boundary-wand` | film | `BoundaryWandInput` | `BoundaryWandOutput` | | |
| `resolve-pins` | film | `PinInput` | `PinOutput` | | |
| `chain-pass` | spine | `{ results: FilmPassResult[], options: ChainOptions }` | `ChainAnswer` | yes | |
| `check-case` | spine | `CaseFileV1` | `CheckFinding[]` | | yes |
| `pack-case` | spine | `OpenedCase` | `Uint8Array` (zip) | | |
| `unpack-case` | spine | `{ bytes }` | `OpenedCase` | | |
| `make-up-spine` | spine | `MadeUpRecipe` | `MadeUpSpine` | | |
| `compare-with-answer-key` | spine | `{ marks: FilmMarks[], key }` | `AnswerKeyComparison` | | yes |

`OpenedCase` (`{ case, films: FilmPng[] }`) and `MadeUpSpine`
(`{ films: FilmPng[], answerKey }`) are generated shapes like every other;
only `OpenedFilm`, which the film worker puts together from two engine
calls, is declared in `job-table.ts`.

Notes on inputs:

- A film's desk keeps its pixels as they were opened, described by the
  `values` in its facts. The case's record of the film can differ only in
  the user's brightness flip and the blanked boxes, so every job that
  takes the record reads the pixels through it: flipped where the
  record's `values.inverted` differs from the desk's, blanked boxes zero
  (every pixel a box reaches into, as `PxRect` defines).
- `open-film` takes the saved record for a case PNG (`null` for a dropped
  file). The PNG must match its `image`, and the desk takes the source,
  kept tags, values and flags from the record, so a reopened DICOM film
  keeps its spacing tags and shutters, and its pixels are exactly what
  the record describes.
- `encode-png` takes the film's record, so the PNG's values are exactly
  what `values` describes, blanked boxes zero. A desk opened from a case
  PNG writes the same pixels back until the record changes.
- `film-pass` takes the film's record: its geometry, its brightness and
  its blanked boxes, which are left out of every map. `runAutoPass` sends
  each film of `AutoPassInput.films`.
- `place-points` takes the first free vertebra id, since ids are unique
  across the case and never reused. `runAutoPass` places points film by
  film. Each film starts one above the highest id given to any earlier
  film in the pass, and at `AutoPassInput.firstVertebraId` while none has
  been.
- `scale-from-known-object` takes where the user pressed (the object's
  centre), where they let go (on its rim, the circle fit's starting
  radius), and the object's true diameter.
- The thumbnail is an 8-bit greyscale PNG, at most 256 pixels on its
  longest side.

## Exports

`engine/wasm-bridge` is the only crate that knows about the browser. It
has one export file per engine crate, named after the crate, and every
export is thin: convert the shapes, call one engine function, convert
back. `FilmDesk` is a JavaScript class; each export file adds its own
`impl FilmDesk` block, so one class spans files owned by different
slices.

| File | Exports | Errors |
|---|---|---|
| `info.rs` | `engine_info() → EngineInfo`; `last_panic() → string \| undefined` | none |
| `shapes.rs` | `read_case_json(text) → CaseFileV1`; `write_case_json(case) → string` | `case-*` from strict reading; `engine-out-of-sync` |
| `film_reader.rs` | `FilmDesk.open(bytes, kind) → FilmDesk`; `FilmDesk.open_saved(png, film) → FilmDesk`; `.facts() → FilmFacts`; `.thumbnail(maxSide) → Uint8Array`; `.display_pixels() → Uint16Array`; `.free()` | `film-*` |
| `film_geometry.rs` | `FilmDesk.sense_geometry() → GeometryGuess`; `.scale_from_ruler(a, b, mm)` and `.scale_from_known_object(near, edge, diameterMm) → ScaleResult` | `geometry-*`, `scale-*` |
| `auto_pass.rs` | `FilmDesk.start_film_pass(film) → FilmPass`; `FilmPass.advance(workUnits) → Step`; `FilmPass.finish() → FilmPassResult`; `ChainPass.start(results, options)`, `.advance()`, `.finish() → ChainAnswer`; `FilmDesk.place_points(chain, firstVertebraId) → FilmMarks` | `pass-*` |
| `toolbox.rs` | `FilmDesk.tool_preview(input)`, `.region_wand(input)`, `.boundary_wand(input)`, `.resolve_pins(input)` | `tool-*` |
| `case_checks.rs` | `check_case(case) → CheckFinding[]` | `case-invalid-value` |
| `case_file.rs` | `pack_case(opened) → Uint8Array`; `unpack_case(bytes) → OpenedCase`; `FilmDesk.encode_png(film) → Uint8Array` | `case-*` |
| `made_up_spines.rs` | `make_up_spine(recipe) → MadeUpSpine` | `made-up-unsatisfiable` |
| `scorekeeper.rs` | `compare_with_answer_key(marks, key) → AnswerKeyComparison` | `case-invalid-value` |

Any export can also fail with `engine-out-of-memory` or end in a panic
(reported as `engine-crashed`), and refuses a value from JavaScript that
does not match the shapes (a wrong type, an unknown field, or a field left
out) with `engine-out-of-sync`.

`info.rs` and `shapes.rs` are built. Each other file holds its
documentation until the slice that owns its crate fills it.

## How data crosses

- **Structured values** cross as plain JavaScript objects through
  serde-wasm-bindgen, set up once in `engine/wasm-bridge/src/crossing.rs`:
  maps as objects, `None` as `null`. Shapes use only `f64`, `f32`, `u32`,
  `u8`, `bool`, `String`, `Vec`, `Option`, string-keyed maps and
  internally tagged enums: no `u64`, no `HashMap`. So the JavaScript
  object, the generated TypeScript type and `case.json` are the same
  shape.
- **Bytes** inside a value arrive as a fresh `Uint8Array`. Float payloads
  inside them are little-endian `f32`; `engine-room/payloads.ts` views
  them as `Float32Array`. `FilmPassResult.strip` and `FilmChain.chain`
  are the engine's own encodings, which the app only forwards.
- **Bulk pixels** return as owned vectors, copied into fresh typed arrays.
  Nothing hands out a view into wasm memory, because such views die when
  memory grows.
- **Errors** leave an export as a thrown plain `EngineError` object.

## Engine info

```ts
type EngineInfo = { engineVersion: string; shapesFingerprint: string; caseFormat: string; caseVersion: number };
```

`shapesFingerprint` is the SHA-256 of the shapes crate's source, compiled
into the engine and written to `app/src/shapes/generated/fingerprint.ts`.
See [worker messages](worker-messages.md) for the handshake.
