# Pixel space

Every point the engine computes, the app draws and the case file stores is
in **original film pixels**.

- **x** goes right (the column), **y** goes down (the row).
- The **centre** of the top-left pixel is (0, 0). Pixel (i, j) covers the
  square from (i − 0.5, j − 0.5) to (i + 0.5, j + 0.5).
- A point is `Px = [x, y]`, two JSON numbers. A box is `PxRect { left,
  top, right, bottom }`, edges included.
- Where a box stands for pixels (a blanked box, a mask), it covers every
  pixel it reaches into: pixel (i, j) when `left < i + 0.5`,
  `i − 0.5 < right`, `top < j + 0.5` and `j − 0.5 < bottom`. A box that
  only meets a pixel's edge does not cover it. A blanked box stays inside
  the film's outer edges, (−0.5, −0.5) to (width − 0.5, height − 0.5).
- Every angle is measured in film pixel axes too: from +x (right) toward
  +y (down). A stage working on a turned grid turns its angles back with
  its positions.

## Films are never changed

Films are never rotated, flipped, cropped or resized, not in memory and
not in the case file.

- **Upright** is a view transform: `quarterTurns` (0 to 3, clockwise) says
  how the app turns the film on screen. The pixels stay as they were.
- **Mirroring** is a view transform too, and only for looking.
- `imageRight` names the patient direction that points to screen right
  **after** the quarter turns.
- A stage that works on a turned, shrunk or straightened copy (a working
  grid, the unrolled strip) maps every result back to original film
  pixels before it leaves the stage. Preview tiles carry an origin and two
  step vectors in film pixels for the same reason.

## The half pixel is added once

Drawing needs pixel edges, so somewhere a half pixel must be added. That
happens in exactly one place, `app/src/film-viewer/film-space.ts`, which
defines the transform from film pixels to the screen. Nothing else adds or
subtracts a half pixel.

Some sources measure from the outer corner of the top-left pixel (pixel
centres at i + 0.5). Converting is an exact half-pixel shift; do it at the
edge where such a source is read, never inside the engine.

## Millimetres

Millimetres exist only through a film's geometry. `mmPerPx` is the
**horizontal** (column) spacing; `mmPerPxVertical` is set only when pixels
are not square. DICOM writes spacing as row spacing then column spacing
(PS3.3 section 10.7.1.3), so its first value is the vertical one. A film
without a known scale still supports every angle; lengths wait for a
scale.

## Where it lives

- `engine/shapes/src/pixels.rs`: `Px`, `PxRect` and the rule itself.
- `app/src/film-viewer/film-space.ts`: the view transform and the one half
  pixel.
