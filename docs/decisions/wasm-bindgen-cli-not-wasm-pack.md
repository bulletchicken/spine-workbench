# The wasm-bindgen tool directly, not wasm-pack

**Decided:** `cargo chores wasm` builds the engine in two visible steps:
`cargo build -p wasm-bridge --target wasm32-unknown-unknown`, then
`wasm-bindgen --target web` into `app/src/engine-room/wasm-build/`. The
`wasm-bindgen` command-line tool is installed at exactly the version of
the `wasm-bindgen` crate pinned in `Cargo.toml`.

## Why

- The glue the tool writes must match the crate exactly, or the module
  and its JavaScript disagree. Pinning one version in one place and
  checking it (`cargo chores versions`) is simpler than trusting a
  wrapper to pick it.
- Two steps we can read beat a wrapper whose upkeep is uncertain and
  which adds its own downloads and defaults.
- `wasm-opt` is left out for now: one less install. It can be added if
  the speed and size scores ask for it.

## Consequences

- A contributor installs the tool once:
  `cargo install wasm-bindgen-cli --version =<crate version> --locked`.
  CI does the same and caches it.
- `cargo chores versions` fails if the crate and the tool differ, or if
  `Cargo.lock` ever holds two versions of the crate.
- Release builds use fat LTO and one codegen unit; the quicker `wasm-dev`
  profile keeps debug assertions on for development.
