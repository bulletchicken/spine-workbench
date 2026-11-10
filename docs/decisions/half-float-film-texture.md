# The film goes to the GPU as a half-float texture

**Decided:** the engine produces a normalised copy of each film as 16-bit
floats (`FilmDesk.display_pixels()`, half-float bit patterns, with
MONOCHROME1 already inverted), and the film viewer uploads it as a WebGL2
`R16F` texture. Window, level, inversion, turning and mirroring happen in
the shader.

## Why

- `R16F` is filterable in core WebGL2, so zooming is smooth without
  extensions.
- An integer texture (`R16UI`) cannot be filtered. Splitting 16 bits into
  two 8-bit channels interpolates wrongly between pixels. An 8-bit
  picture would lose window and level on 12- and 16-bit films.
- Half floats halve the memory of 32-bit floats; up to four full-length
  films stay within what a browser tab can hold.

## Consequences

- An 11-bit mantissa is ample for an 8-bit display after window and
  level, but a raw value read back from the texture is approximate. Exact
  values come from the engine.
- Past 1:1 zoom the shader switches to nearest sampling, so single
  pixels stay visible.
- Half-float support has to be checked on Safari's WebGL2 when the
  viewer is built.
