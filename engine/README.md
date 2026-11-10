# engine

All the computing, in Rust. Nothing here knows a browser exists except
`wasm-bridge`. Every crate builds and tests natively; the browser runs the
same code compiled to WebAssembly.

## Crates, in the order a film travels

| Crate | Does |
|---|---|
| `shapes/` | every shared data shape: the single source of truth for Rust, TypeScript and the case file |
| `darkroom/` | image filters every step shares; knows nothing about spines |
| `forward-model/` | the one model of a vertebra and how it lands on a film |
| `made-up-spines/` | spines and films with known answers |
| `film-reader/` | bytes in, one greyscale film out; DICOM tags not on the keep-list dropped |
| `film-geometry/` | which way up, which way it faces, how big a pixel is |
| `auto-pass/` | the watchable top-down vertebra search |
| `toolbox/` | the maths behind the region wand, the boundary wand and pins |
| `case-checks/` | is the spine complete and in order? |
| `case-file/` | one zip per case, read strictly |
| `scorekeeper/` | choosing methods with numbers; never touches the disk |
| `command-line/` | the `spine` binary: the same engine, natively |
| `wasm-bridge/` | the only crate that knows about the browser |

## Who depends on whom

Every crate uses `shapes`. Beyond that, each crate uses only crates above
it in the story:

```
darkroom        ──▶ shapes
forward-model   ──▶ shapes
made-up-spines  ──▶ darkroom, forward-model
film-reader     ──▶ darkroom
film-geometry   ──▶ darkroom, film-reader
auto-pass       ──▶ darkroom, film-reader, forward-model
toolbox         ──▶ darkroom, film-reader, auto-pass
case-checks     ──▶ shapes
case-file       ──▶ shapes
scorekeeper     ──▶ forward-model, made-up-spines, film-reader, auto-pass, toolbox, case-file
command-line    ──▶ film-reader, film-geometry, auto-pass, toolbox, case-checks,
                    case-file, made-up-spines, scorekeeper
wasm-bridge     ──▶ the same crates as command-line, never command-line itself
```

Tests of `film-geometry`, `auto-pass` and `toolbox` also use
`made-up-spines`. `cargo chores seams` keeps the rules that matter: only
`wasm-bridge` reaches browser crates, no crate reaches a network crate,
and the bridge never reaches `command-line`, `chores` or command-line
tools.

## The patient frame in one picture

```
              z up (head)
              │
              │   y to the patient's left
              │  ╱
              │ ╱
              │╱
  ────────────●──────────── x forward (anterior)
          origin: centre of the underside of L5

  front film, imageRight = patient-left:   u = u0 + y / mmPerPx,  v = v0 − z / mmPerPx
  side film,  imageRight = anterior:       u = u0 + x / mmPerPx,  v = v0 − z / mmPerPx
```

Points on films are in original film pixels, x right, y down, the centre
of the top-left pixel at (0, 0). See `docs/conventions/` for the full
rules.

## Rules every crate keeps

- `#![forbid(unsafe_code)]`, except in `wasm-bridge`, whose generated code
  needs it.
- No clocks, threads, files, environment or network in engine code, and
  transcendental maths from `libm`, not the standard library:
  `clippy.toml` enforces both. Only `command-line` and `chores` read files.
- Randomness only from seeded generators, so every run can be repeated.
- Package name = folder name; `publish = false`. Every dependency is
  pinned once, in the workspace `Cargo.toml`.
