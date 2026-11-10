# The Rust engine runs in plain workers

**Decided:** all computing is Rust, compiled to one WebAssembly module and
run in plain module workers: one film worker per loaded film (up to four)
and one spine worker. No WebAssembly threads, no `SharedArrayBuffer`.

## Why

- Image work on a full-length film takes long enough to freeze a page.
  Workers keep the screens responsive and let the four films' passes run
  side by side.
- WebAssembly threads need cross-origin isolation headers and a nightly
  toolchain for the standard library. Plain workers need neither, and
  give the parallelism that matters here: one film per worker.
- One film per worker makes a film's state natural to keep: the worker
  holds a `FilmDesk` with the decoded pixels and cached results, and
  nothing else.
- The same Rust runs natively (`cargo spine`, the tests, the
  scorekeeper), so every result can be checked without a browser.

## Consequences

- Workers never talk to each other. The main thread conducts the auto
  pass: film passes in parallel, the small results to the spine worker,
  points placed back on each film.
- The engine is stateless apart from each desk's pixels and caches. Marks
  live on the main thread and every request carries the film's current
  marks, so a crashed worker loses nothing: it is restarted from the
  cached module and its film reopened.
- Only `engine/wasm-bridge` knows a browser exists; `cargo chores seams`
  fails if any other crate reaches a browser crate.
