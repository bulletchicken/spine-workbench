# Placed bodies

Known answers for the 3D view's placing code: a vertebra, and where its
endplate centres and body corners are in the patient and on each of four
films.

`placed-bodies.json` was written by the engine's forward model, not
worked by hand: for each vertebra, `Endplates::of` gives the two
endplates; the body's corner on a film is the rim point picked by the
closed form of `corners.rs` through `ParallelCamera::jacobian`, checked
there to land where `Camera::corners`, `project::front_marks` and
`project::side_marks` put the corner; and every point is landed on every
film by `ParallelCamera::project`. All of these are in
`engine/forward-model`. The hand-worked fixtures for those functions
themselves are in `../corners/`.

`app/src/spine-3d/placing/known-answers.test.ts` reads the file. It
shapes every shipped bone to each vertebra, stands it in the patient,
and fails if one of the bone's own landmarks is further from its answer
than the file's tolerance.

## Format

```json
{
  "what": "what the file holds",
  "toleranceMm": 0.05,
  "tolerancePx": 0.05,
  "films": { "front": { "kind": "front", "imageRight": "patient-left", "quarterTurns": 0, "size": [1250, 2776], "mmPerPx": 0.28, "mmPerPxVertical": null, "origin": [640, 2350] } },
  "cases": [
    {
      "what": "what this vertebra shows",
      "vertebra": { "centre": [0, 10, 100], "rotation": [0, 0, 0], "height": 20, "wedgeFront": 0, "wedgeSide": 0, "topHalfWidth": 15, "topHalfDepth": 10, "bottomHalfWidth": 15, "bottomHalfDepth": 10 },
      "points": { "topCentre": [0, 10, 110], "supL": [0, 25, 110] },
      "on": { "front": { "topCentre": [675.7, 1957.1], "supL": [729.3, 1957.1] } }
    }
  ]
}
```

| Field | Meaning |
|---|---|
| `films` | four films, each as in `../corners/README.md`: `front` (facing the patient's left), `side` (facing anterior), `mirroredFront` (facing the patient's right) and `turnedSide` (facing posterior, stored a quarter turn round, with pixels that are not square) |
| `vertebra` | the thirteen numbers, in the patient frame (mm and radians) |
| `points` | ten points in the patient frame, mm: `topCentre` and `bottomCentre`, the body's four corners as a front film shows them (`supL`, `supR`, `infL`, `infR`) and as a side film shows them (`supA`, `supP`, `infA`, `infP`) |
| `on` | where each of the ten points lands on each film, in original film pixels |

The corners are those of the `front` and `side` films. A film with
square pixels shows the same ones, mirrored or not; `turnedSide`, whose
pixels are not square, has slightly different corners of its own, and
the ten points are landed on it as plain points.

The twelve vertebrae: straight; a side tilt; a forward lean; a twist; a
front wedge; a side wedge; unequal endplates; tilt, lean and twist
together; those with both wedges and unequal endplates; a large twist
with both wedges; a small upper thoracic body; a large lumbar body.
