# Watch frames

The frames the step 3 sweep animation plays: the contract between the auto
pass and the screen. Frozen: it changes only through a contract-change
pull request. The source is `engine/shapes/src/watch_frames.rs`; the
TypeScript is the generated `WatchFrame` union.

## Rules

- **Real output at fixed checkpoints.** Every frame is something the
  engine actually computed, emitted at a checkpoint in the algorithm. No
  frame depends on how the work was sliced, so a native run and a browser
  run produce the same sequence, and the last frame equals a direct run's
  answer. The auto pass's tests hold it to both
  (`watching_does_not_change_the_answer.rs`,
  `frames_do_not_depend_on_slice_size.rs`).
- **Steppers, not callbacks.** Rust never calls JavaScript. A pass is a
  state machine: `advance(workUnits)` does a bounded slice and returns a
  `Step { done, stage, fraction, frames }`.
- **Every frame names its `film` and `stage`.** Every position is in
  original film pixels.
- **Pacing is the app's job.** The worker sends progress at most ten
  times a second. The step 3 frame player buffers frames, gives each
  stage a minimum dwell, offers skip, replay and scrubbing, and falls
  back to cross-fades under reduced motion.
- **Every frame is kept except two kinds,** which may be thinned to
  thirty a second, newest kept: `joint-progress` frames, and `evidence`
  frames with `final: false` whose `map` is not `wavefront`. A frame held
  back is sent or dropped before any later frame of its pass is sent, so
  each film's frames, and the chain pass's, arrive in the order they were
  emitted. The film passes run in parallel, so their frames may
  interleave. The player replays what it was sent, so a dropped strip
  block would leave a hole in the strip and a dropped sweep frame a gap
  in the search.
  `WatchFrame::is_keyframe` is false for exactly those two.
- **Outlines may move at the end.** The best chain is known only once the
  sweep finishes, so an outline already shown may shift when the final
  outlines arrive; the app glides it into place.

## Stages

```
evidence → centre-line → strip → chain → naming → confidence
└──────── film pass, per film ───┘ └────── chain pass, all films ──────┘
```

## Frame kinds, in the order they arrive

| Kind | Stage | Carries |
|---|---|---|
| `stage-started` | any | the stage that begins |
| `evidence` | evidence, centre-line | a `PreviewTile` of one map (`map`), its place in the film's sequence (`step` of `steps`), whether it is the finished map, and its filter scale in mm when it has one |
| `end-zones` | centre-line | the boxes where the centre line starts and ends, and whether the film is cropped at either end |
| `centre-line` | centre-line | the line from the neck down, coarse first, then `refined: true` |
| `width` | centre-line | the spine's measured edges along the line |
| `strip-shape` | strip | each strip row's centre and normal on the film, and the column spacing: the map for the curved-to-straight morph |
| `strip-rows` | strip | a block of the unrolled strip, head first: one byte per cell and a validity bit per cell |
| `barcode` | strip | the bone and gap profile along the strip, raw and with slow trends removed (little-endian `f32` per row) |
| `sweep` | chain | the level the search has reached, a 256-bin heat line of where it could end, and the best chain so far |
| `template-scored` | chain | one vertebra count's score and chance |
| `joint-progress` | chain | progress when a front and a side film are solved together |
| `chain-locked` | chain | every body found, as endplate lines |
| `levels-named` | naming | the film's `LevelNaming` and the level of each body, top to bottom |
| `confidence` | confidence | each level's confidence |
| `outlines` | confidence | one quad per vertebra, corners in quad order round the outline; the last has `final: true` |
| `warning` | any | something the user should know about the pass, between any two frames |

## Preview tiles

A `PreviewTile` is a small 8-bit picture of a map, placed exactly on the
film. Cell (i, j), i across and j down, is centred at the film point

```
origin + i · stepAcross + j · stepDown
```

The two steps are vectors in film pixels, so a tile computed on a turned
or shrunk working copy still lands where it belongs. `encoding` says what
a byte means:

| Encoding | Byte |
|---|---|
| `unit` | a 0 to 1 value: `round(255 · v)` |
| `direction` | an angle in (−π, π]: `round(255 · (θ + π) / 2π) mod 256` |
| `axial` | a line angle in [−π/2, π/2): `round(255 · (θ + π/2) / π)` |
| `mask` | 0 or 255 |
| `index` | an index into a list (a scale, a shape) |
| `signed-pair` | two bytes per cell: the ridge value, then the valley value |

Byte fields cross into JavaScript as fresh `Uint8Array`s and travel in the
`postMessage` transfer list.
