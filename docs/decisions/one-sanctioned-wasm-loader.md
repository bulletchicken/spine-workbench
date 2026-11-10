# One sanctioned loader for our own .wasm

**Decided:** exactly one file in the app may make a network request:
`app/src/engine-room/load-engine.ts`, which fetches the engine's own
`.wasm` from the page's origin, compiles it once and posts the compiled
`WebAssembly.Module` to every worker. Workers start it with
wasm-bindgen's `initSync({ module })` and never fetch.

## Why

- The app makes no network calls; the lint rule that bans them must
  still let the engine load. One named exemption is easy to audit; a
  loader hidden in a plugin is not.
- wasm-bindgen's own loader fetches the file itself, from every worker.
  Compiling once on the main thread and posting the module means one
  request, one compile, and no fetch the lint rule cannot see.
- No `vite-plugin-wasm` and no top-level-await plugin: both would add a
  fetch outside our code.
- Vite's module-preload polyfill also fetches scripts itself, so it is
  turned off (`build.modulePreload.polyfill = false`). The bundle then
  holds a single `fetch` call, the loader's.

## Consequences

- `eslint.config.js` exempts that one file, for `fetch` only. A test
  fails if the exemption list ever has a second entry or another file's
  network rules are weakened.
- The loader checks the URL's origin is the page's own before fetching,
  and uses streaming compilation only when the file is served as
  `application/wasm`.
- `nothing-leaves.spec.ts` asserts the page makes exactly one fetch: the
  hashed `.wasm`.
- Posting a compiled module needs Safari 16.4 or later, which the 128-bit
  SIMD build needs anyway. If a browser could not post modules, each
  worker would compile from the same URL instead; that would be a
  contract change to the boot message.
