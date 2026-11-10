# libm for maths that matches everywhere

**Decided:** engine code takes every transcendental function (`sin`,
`cos`, `atan2`, `exp`, `ln`, `powf`, `hypot` and the rest) from the
pure-Rust `libm` crate, never from the standard library's float methods.
`clippy.toml` bans the standard ones, and `mul_add`.

## Why

- Natively, the standard library calls the system's maths library; in
  WebAssembly it uses a Rust port. The two can differ in the last bit,
  and a last-bit difference can flip a comparison in a search and change
  an answer.
- Native scores must describe what the browser does. With one
  implementation everywhere, a made-up spine gives the same answer in
  `cargo spine` and in the app.
- `sqrt` and plain arithmetic are exact under IEEE 754 on both targets,
  so they need nothing.

## Consequences

- Results match bit for bit wherever no dependency intervenes. `rustfft`
  and `nalgebra` use their own arithmetic, and SIMD can change the order
  of sums, so parity tests say where they are exact and where they allow
  a stated tolerance.
- Along with libm: maps with a defined order (`BTreeMap`), floats sorted
  with `total_cmp`, loops in a fixed order, ties broken explicitly, and
  randomness only from seeded generators.
