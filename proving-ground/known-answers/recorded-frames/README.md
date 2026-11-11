# Recorded auto-pass frames

`cargo spine frames` records the front film of right-thoracic-40, seed 1,
tier t0. `cargo spine frames --check` recomputes it and fails on any byte
drift. The recipe supplies the film's true geometry, but no vertebral
positions enter the search. These are observations of the current pass,
not an accuracy target.

`frames.json` holds the input record, the returned marks, every fixed
checkpoint, the film result (with its strip bytes stored separately), and
the chain answer. Byte payloads live in `frame-payloads.bin`. Their JSON
fields give `byteOffset`, `byteLength`, decoded `length`, and `encoding`:
`raw` holds literal bytes; `runs` holds pairs of a nonzero count (at most
255) followed by its byte. Replaying restores each field to a fresh
`Uint8Array`, exactly the original frame shape. `front.strip` is
the native film pass's opaque baton. It lets parity run the chain with
identical input bytes, requiring exact results independently of FFT
roundoff in the preceding evidence filters.

`proving-ground/parity/native-vs-wasm.test.ts` regenerates the film in the
browser build, checks the recorded checkpoints and marks, and runs that
same native strip through the browser chain pass. No real films or labels
are stored here.

Parity requires exact chain results and frames for the shared native strip.
The complete run allows `1e-5 + 2e-5 × |reference|` for floats affected by
FFT arithmetic, one byte for quantised previews, and four bytes only for
Hessian/valley responses at most 8/255. Those weak responses amplify FFT
cancellation through the contrast floor; stronger responses still allow
only one byte. Masks, indices, frame kinds, levels and counts stay exact.
