# spine-3d

The spine in three dimensions: a fitted spine drawn with real bone
shapes, and the films it was fitted from standing in the same space. It
takes a fit, the bone shapes and (if wanted) the films' pictures as
inputs, and draws. It holds none of the case, reaches no engine and
fetches nothing. No screen mounts it yet.

The bones drawn are one adult's, each shaped to its fitted vertebra: a
generic bone shaped to the fit, never the patient's own.

## Folders

| Folder | Holds |
|---|---|
| `bones/` | the bone shapes: the two files `bone-prep` writes, the reader, and the way the app gets them without a fetch |
| `placing/` | where things go, as plain arithmetic with no drawing library: a bone shaped to a fitted vertebra, a vertebra stood in the patient, patient axes as the scene's, a patient point on a film and back, a bone pressed flat on its film |
| `looking/` | where the spine is seen from, as plain arithmetic: a view, the lens from no perspective to a narrow one, the named views, fitting the canvas, the view along a film, turning by hand |
| `scene/` | the drawing, with three.js: the stage, a spine and its bones, a film's sheet, the shadow, the camera, the hand's turning |
| `motion/` | small helpers for whoever animates the view from one progress number |
| `view/` | `Spine3D`, the part a screen places, and the gallery pages |
| `made-up-case/` | one made-up spine fitted from two films and from front corners alone, for tests and gallery pages |

`placing/` and `looking/` import nothing that draws, so every claim about
where a bone lands is tested as numbers.

## How a bone gets to its place

1. **Shaped.** `placing/shape-bone.ts` moves every point of the bone so
   that its two endplates become the fitted vertebra's: the same centres,
   the same four half-sizes and both wedges, with the bone's own wedges
   taken out. Between the plates the two are blended, so the walls stay
   smooth. The arch behind the body is not stretched with the body's
   depth or height: it is scaled evenly by the ratio of widths, about the
   place the pedicles leave the body. The header of that file says how
   each part is done.
2. **Stood.** `placing/place-vertebra.ts`: `patient = centre + R × own`,
   with `R` made from the fit's rotation vector exactly as the engine
   makes it.
3. **Renamed.** `placing/scene-axes.ts`: patient x (forward), y (left), z
   (up) become the scene's Z, X and Y. That is a turn, never a mirror.

A film is stood in the same space by `placing/film-camera.ts`, which is
the engine's parallel camera in TypeScript. `placing/fitted-film-camera.ts`
takes its spacing and origin pixel from `fit.frontFilm` or `fit.sideFilm`,
and its facing, quarter turns and stored size from the film. The side
film's reported spacing already includes `sideScale`. It is never fitted
to the bones.

## What holds it to the engine

| Test | Holds |
|---|---|
| `placing/known-answers.test.ts` | every bone, shaped to each of twelve vertebrae and stood in the patient, has its endplate centres and body corners where the engine's forward model puts the vertebra's (`proving-ground/known-answers/spine-3d/`), in space and on four films |
| `placing/film-camera.test.ts` | the camera lands the hand-worked corner fixtures (`proving-ground/known-answers/corners/`) |
| `placing/bones-sit-on-their-outlines.test.ts` | on both fits of a made-up spine, each bone's body corners land on the fit's own to a tenth of a pixel, and on the marks the fit was given to within the fit's residual; a view along the film draws them on those pixels |
| `placing/scene-axes.test.ts` | the patient's left is on the right of the picture in the front view |
| `looking/lens.test.ts` | the matrices handed to the GPU draw every point where the plain rule does |

## Nothing moves by itself

Every value is set from outside and drawn on the next frame: the view,
and for each bone its pose (pressed flat on its film to where the fit
stands it), reveal, fill, strength, tint and see-through lines; for the
spine its depth, its own fill, the end both fills start from (the top,
T1 first, unless told otherwise), the colour of its see-through lines
(ink for the page, white for over a film), place, turn and shadow; for a film its
strength and hinge. A transition is whatever its caller makes of one
progress number (`motion/easing.ts` has the curve and the helpers), and
`settle()` jumps to rest. Two things are the exception: a spine thrown by
hand glides to a stop, and `Spine3D` travels to a named view over 600 ms
unless motion is reduced or it is told to jump.

The stage draws only when something changed, on the app's one animation
loop.

## Using it

```tsx
const bones = await loadBones(); // bones/load-bones.ts: a script of its own, asked for once
<Spine3D fit={fit} bones={bones} films={{ front: { geometry, size, pixels } }} ref={handle} />;

const { stage, spine } = /* from onReady, or handle.stage() and handle.spine() */;
spine.set({ fill: 0.4, cage: 1 }, 'T7');           // one bone
spine.set({ strength: 0.55 });                      // every bone
spine.setSpine({ depth: 0 });                       // flat against its film
stage.setView(stage.viewAlong('front', shown)!);    // on the film's own pixels
stage.patientToCanvas(point, spine);                // for anything drawn over the canvas
stage.pick(x, y);                                   // the level under the pointer
```

What was measured and what was taken from a typical spine is the fit's
to say: `spine.about(level)` hands back the fitted vertebra with its
`inferred` flags and its `sd`, unstyled. `cornerTiltRad` is the tilt read
from the front corners; `angles.sideTiltRad` is the model's tilt, which
also moves with typical lean and twist. The `OneFilm` gallery page prints
both beside the inferred flags, length status and typical curve size.
It identifies typical values as such. The corner tilt has a first-order band propagated from the input corner
spread through the line between the side midpoints; model angles carry
their reported standard deviations.

## Rules

- No loaders, no orbit controls, no fetch: bones come from
  `bones/load-bones.ts`, a film's picture from its caller.
- No folder here takes another area's name (`shapes`, `marks`, `steps`,
  `gallery` and so on): the import rule matches an area's folder at any
  depth.
- `scene/look.ts` holds every colour and strength of the stage. They are
  the design's values; keep them equal to it.
