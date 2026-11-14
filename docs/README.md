# docs

The explanations. Code says what happens; these pages say what the
rules are and why they were chosen.

| Folder | Holds |
|---|---|
| [`conventions/`](conventions/) | the rules every slice shares: coordinates, signs, names |
| [`contracts/`](contracts/) | the frozen boundaries between the engine, the workers and the app |
| [`decisions/`](decisions/) | one short record per choice, named by topic |

## Conventions

- [Pixel space](conventions/pixel-space.md): original film pixels, x right,
  y down, the top-left pixel's centre at (0, 0); films are never changed.
- [Patient frame](conventions/patient-frame.md): millimetres, x forward, y
  to the patient's left, z up, origin under L5.
- [Signs and wedges](conventions/signs-and-wedges.md): the 13 numbers of a
  vertebra and the sign of each.
- [Mark and level names](conventions/mark-and-level-names.md): `supL` to
  `infP`, landmarks, and levels C7 to L6.
- [Naming in code](conventions/naming-in-code.md): folders, files, film
  kinds and spelling.

## Contracts

Each changes only through its own small contract-change pull request.

- [Engine API](contracts/engine-api.md): every export, job and client
  call, with inputs, outputs, errors and the worker that runs it.
- [Worker messages](contracts/worker-messages.md): the protocol between
  the main thread and the workers.
- [Watch frames](contracts/watch-frames.md): what the sweep animation
  receives.
- [Case file, version 1](contracts/case-file-v1.md): the saved file, with
  its generated [JSON Schema](contracts/case-file-v1.schema.json).

## Decisions

- [The Rust engine runs in plain workers](decisions/rust-engine-in-plain-workers.md)
- [One sanctioned loader for our own .wasm](decisions/one-sanctioned-wasm-loader.md)
- ['wasm-unsafe-eval' and the policy header for workers](decisions/wasm-unsafe-eval-and-worker-headers.md)
- [The wasm-bindgen tool directly, not wasm-pack](decisions/wasm-bindgen-cli-not-wasm-pack.md)
- [Rust owns decoding and the case file](decisions/rust-owns-decoding-and-the-case-file.md)
- [Films are saved as lossless PNG](decisions/films-saved-as-png.md)
- [Stepped passes for the watchable sweep](decisions/stepped-passes-for-the-watchable-sweep.md)
- [The film goes to the GPU as a half-float texture](decisions/half-float-film-texture.md)
- [libm for maths that matches everywhere](decisions/libm-for-matching-maths.md)
- [Placements are compared over versions](decisions/placements-are-compared-over-versions.md)

A new decision gets its own file, named by its topic rather than a
number, so pull requests written in parallel never collide.
