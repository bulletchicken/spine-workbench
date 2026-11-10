# Corner fixtures

Known answers for the forward model: a vertebra, a film, and where the
vertebra's marks must land on that film. Every expected value was worked
out by hand from the conventions alone (`docs/conventions/patient-frame.md`,
`signs-and-wedges.md`, `pixel-space.md`), never by running the model, and
is written below so anyone can check it with pencil and paper.

`engine/forward-model/tests/corner_fixtures_hold.rs` reads every `.json`
file here and fails if any mark is further from its expected position
than the file's tolerance. Later work reads the same files: the made-up
spines, the scorekeeper and the Wave 2 fit can each check that they place
marks exactly as the forward model does.

## Format

Each file is one JSON object. The format is stable: a field is never
renamed or given a new meaning, and a new kind of camera is added as a
new field, never by changing an old one.

```json
{
  "what": "what the cases in this file have in common",
  "tolerancePx": 1e-9,
  "cases": [
    {
      "what": "what this case shows",
      "vertebra": {
        "centre": [0, 10, 100],
        "rotation": [0, 0, 0],
        "height": 20,
        "wedgeFront": 0,
        "wedgeSide": 0,
        "topHalfWidth": 15,
        "topHalfDepth": 10,
        "bottomHalfWidth": 15,
        "bottomHalfDepth": 10
      },
      "film": {
        "kind": "front",
        "imageRight": "patient-left",
        "quarterTurns": 0,
        "size": [2000, 4000],
        "mmPerPx": 0.5,
        "origin": [1000, 3000]
      },
      "expect": {
        "supL": [1050, 2780], "supR": [990, 2780],
        "infL": [1050, 2820], "infR": [990, 2820],
        "pedL": [1043.4, 2792.8], "pedR": [996.6, 2792.8]
      }
    }
  ]
}
```

| Field | Meaning |
|---|---|
| `tolerancePx` | how far, in film pixels, a computed mark may lie from its expected position. `1e-9` everywhere: the expected values carry 16 or more significant figures, and the model's own rounding is about `1e-12` px at these coordinates |
| `vertebra` | the thirteen numbers, in the patient frame (mm and radians), with the same field names as a made-up spine's saved pose (`VertebraPose`, without `level`) |
| `film.kind` | `front`, `side`, `bend-left` or `bend-right`: the film kinds of `docs/conventions/naming-in-code.md`, read as the shapes crate's `FilmKind`. It says which marks the case expects. Bending films are front views, so `imageRight` is `patient-left` or `patient-right` for `front`, `bend-left` and `bend-right`, and `anterior` or `posterior` for `side`. Which way the patient bends is in the vertebra, not the film, so the two bending kinds project alike |
| `film.imageRight` | the patient direction pointing to screen right once the film is upright, as in a film's saved geometry |
| `film.quarterTurns` | clockwise quarter turns that show the film upright |
| `film.size` | the film's width and height in pixels as stored |
| `film.mmPerPx` | mm per pixel along a stored row, as in a film's saved geometry (`FilmGeometry.mmPerPx`) |
| `film.mmPerPxVertical` | mm per pixel down a stored column, as in a saved geometry: given only when pixels are not square, and `mmPerPx` when left out |
| `film.origin` | the upright pixel `(u0, v0)` where the patient origin lands |
| `expect` | `front`: `supL`, `supR`, `infL`, `infR`, `pedL`, `pedR`; `side`: `supA`, `supP`, `infA`, `infP`; `bend-left` and `bend-right`: `sup` and `inf`, each a pair of points ordered patient left, then patient right. Every point is `[x, y]` in original film pixels |

Every film is a parallel camera. Each case lists every mark of its kind.
The reader refuses unknown fields, and missing ones other than
`mmPerPxVertical`, so a misspelt field fails rather than being skipped;
a missing or misspelt mark is named in the message. The example above
is read and checked by the same test, so it stays a working case.

## How the answers were worked out

On an upright film, `u = u0 + across / su` and `v = v0 − z / sv`, where
`across` is `y` (facing patient left), `−y` (patient right), `x`
(anterior) or `−x` (posterior), and `su`, `sv` are the upright pixel's
width and height. Unless a case says otherwise, pixels are 0.5 mm, there
are no turns and the origin lands at (1000, 3000), so
`u = 1000 + 2 · across` and `v = 3000 − 2z`.

A vertebra's endplate centres are `centre ± (h/2)` along its own up axis.
An endplate is an ellipse with half-width `w` across (toward the
vertebra's left) and half-depth `d` front to back. On a film that sees it
edge on, its corners are simply its ends: `centre ± w` across on front
films, `centre ± d` front to back on side films. When the endplate is
turned in its own plane, its ends on the film are not the ends of its
axes: the rim `c + w cos t ê_w + d sin t ê_d` reaches
`√((w ê_w · â)² + (d ê_d · â)²)` along a unit direction `â`, the
half-extent of the ellipse along `â`. Pedicles sit at
`(−1.25 d̄, ±0.78 w̄, +0.18 h)` from the centre in the vertebra's own frame.
`√3 = 1.7320508075688772…`; values with it are written to full double
precision.

### `straight.json`

The vertebra centred at (0, 10, 100) with height 20, half-widths 15 and
half-depths 10, no turn and no wedge. The top endplate is at z = 110
(v = 2780), the bottom at z = 90 (v = 2820). Across the endplate its ends
are at y = 10 ± 15 = 25 and −5; front to back at x = ±10. The pedicles
are at (−12.5, 10 ± 11.7, 103.6), so y = 21.7 and −1.7, and
v = 3000 − 207.2 = 2792.8.

| Case | Expected |
|---|---|
| front, patient left | supL u = 1000 + 50 = 1050, supR u = 1000 − 10 = 990; pedL u = 1000 + 43.4, pedR u = 1000 − 3.4 |
| front, patient right | across is −y: supL u = 950, supR u = 1010; pedL u = 956.6, pedR u = 1003.4. The names stay with the patient; only the sides of the image swap |
| side, anterior | supA u = 1000 + 20 = 1020, supP u = 980 |
| side, posterior | across is −x: supA u = 980, supP u = 1020 |
| bending (`bend-left`), patient right | the front film's corners, as lines: sup = [supL, supR], inf = [infL, infR] |
| front, lower endplate 16 by 11 | the bottom ends move to y = 10 ± 16: infL u = 1052, infR u = 988. The pedicles use the mean sizes, d̄ = 10.5 and w̄ = 15.5: (−13.125, 10 ± 12.09, 103.6), so pedL u = 1044.18, pedR u = 995.82 |
| side, lower endplate 16 by 11 | infA u = 1000 + 22 = 1022, infP u = 978 |

### `turned-films.json`

The same straight vertebra on stored films whose pixels are 0.4 mm along a
row and 0.5 mm down a column, with the origin landing at upright pixel
(1000, 3000). After an odd number of quarter turns the upright pixel is
`su = 0.5` wide and `sv = 0.4` tall; after an even number, `su = 0.4` and
`sv = 0.5`. Upright pixels go back to the stored film by the inverse of
the app's `uprightMatrix`, with `w`, `h` the stored width and height less
one: one turn, `x = v`, `y = h − u`; two turns, `x = w − u`, `y = h − v`;
three turns, `x = w − v`, `y = u`.

| Case | Upright | Stored film |
|---|---|---|
| one turn, front, patient left (stored 4000 by 2000) | supL u = 1000 + 25/0.5 = 1050, v = 3000 − 110/0.4 = 2725; supR u = 990; the bottom at v = 3000 − 90/0.4 = 2775; pedicles at u = 1043.4 and 996.6, v = 3000 − 103.6/0.4 = 2741 | supL (2725, 1999 − 1050 = 949), supR (2725, 1009), infL (2775, 949), infR (2775, 1009), pedL (2741, 955.6), pedR (2741, 1002.4) |
| two turns, front, patient left (stored 2000 by 4000) | supL u = 1000 + 25/0.4 = 1062.5, v = 3000 − 110/0.5 = 2780; supR u = 1000 − 5/0.4 = 987.5; the bottom at v = 2820; pedicles at u = 1000 + 21.7/0.4 = 1054.25 and 1000 − 1.7/0.4 = 995.75, v = 2792.8 | supL (1999 − 1062.5, 3999 − 2780) = (936.5, 1219), supR (1011.5, 1219), infL (936.5, 1179), infR (1011.5, 1179), pedL (944.75, 1206.2), pedR (1003.25, 1206.2) |
| three turns, front, patient left (stored 4000 by 2000) | as for one turn | supL (3999 − 2725, 1050) = (1274, 1050), supR (1274, 990), infL (1224, 1050), infR (1224, 990), pedL (1258, 1043.4), pedR (1258, 996.6) |
| one turn, side, posterior (stored 4000 by 2000) | across is −x: supA u = 1000 − 10/0.5 = 980, supP u = 1020, v = 2725; the bottom at v = 2775 | supA (2725, 1019), supP (2725, 979), infA (2775, 1019), infP (2775, 979) |

### `tilted-and-wedged.json`

A vertebra centred at (0, 0, 100) with height 20, half-widths 15 and
half-depths 10, changed in one way per case. Turns are 30° (`π/6`),
wedges 60° (`π/3`, so each endplate tilts 30°): `sin 30° = ½`,
`cos 30° = √3/2`.

**Tilt**, rotation vector (π/6, 0, 0), which is rotX(30°): `y′ = y cos − z sin`,
`z′ = y sin + z cos`. Front film, patient left.

- Top centre (0, 0, 10) turns to (0, −5, 5√3), bottom centre to
  (0, 5, −5√3). The width axis ŷ turns to (0, √3/2, ½).
- supL = (0, −5 + 7.5√3, 107.5 + 5√3) → (990 + 15√3, 2785 − 10√3);
  supR = (0, −5 − 7.5√3, 92.5 + 5√3) → (990 − 15√3, 2815 − 10√3);
  infL = (0, 5 + 7.5√3, 107.5 − 5√3) → (1010 + 15√3, 2785 + 10√3);
  infR = (0, 5 − 7.5√3, 92.5 − 5√3) → (1010 − 15√3, 2815 + 10√3).
- Pedicle offsets (−12.5, ±11.7, 3.6) turn to
  (−12.5, ±5.85√3 − 1.8, ±5.85 + 1.8√3): pedL → (996.4 + 11.7√3,
  2788.3 − 3.6√3), pedR → (996.4 − 11.7√3, 2811.7 − 3.6√3).

**Lean**, rotation vector (0, π/6, 0), which is rotY(30°): `x′ = x cos + z sin`,
`z′ = −x sin + z cos`. Side film, anterior.

- Top centre (5, 0, 5√3), bottom centre (−5, 0, −5√3). The depth axis −x̂
  (front to back) turns to (−√3/2, 0, ½); the anterior end is
  centre − 10 · depth axis.
- supA = (5 + 5√3, 0, 95 + 5√3) → (1010 + 10√3, 2810 − 10√3);
  supP = (5 − 5√3, 0, 105 + 5√3) → (1010 − 10√3, 2790 − 10√3);
  infA = (−5 + 5√3, 0, 95 − 5√3) → (990 + 10√3, 2810 + 10√3);
  infP = (−5 − 5√3, 0, 105 − 5√3) → (990 − 10√3, 2790 + 10√3).

**Twist**, rotation vector (0, 0, π/6), which is rotZ(30°): `x′ = x cos − y sin`,
`y′ = x sin + y cos`. Front film, patient left.

- The endplates stay level, at v = 2780 and 2820. The turned ellipse
  reaches `√((15 cos 30°)² + (10 sin 30°)²) = √193.75` mm across, so the
  corners are at u = 1000 ± 2√193.75 = 1000 ± √775.
- Pedicle offsets turn to y′ = −6.25 ± 5.85√3: pedL u = 987.5 + 11.7√3,
  pedR u = 987.5 − 11.7√3, both at v = 2792.8.

**Front wedge** π/3. Front film, patient left; then a bending film
(`bend-right`), patient right.

- The top normal is rotX(30°)ẑ = (0, −½, √3/2). Its width axis,
  `normalise(ŷ − (ŷ · n) n)`, is (0, √3/2, ½): the left end 7.5 mm above
  the centre at z = 110, the right end 7.5 mm below. The bottom normal
  (0, ½, √3/2) gives (0, √3/2, −½) about z = 90.
- supL = (0, 7.5√3, 117.5) → (1000 + 15√3, 2765);
  supR = (0, −7.5√3, 102.5) → (1000 − 15√3, 2795);
  infL = (0, 7.5√3, 82.5) → (1000 + 15√3, 2835);
  infR = (0, −7.5√3, 97.5) → (1000 − 15√3, 2805).
  The left side is 35 mm tall, the right side 5 mm.
- A wedge does not move the pedicles: (−12.5, ±11.7, 103.6), pedL u = 1023.4,
  pedR u = 976.6, v = 2792.8.
- On the bending film facing patient right, across is −y:
  sup = [(1000 − 15√3, 2765), (1000 + 15√3, 2795)],
  inf = [(1000 − 15√3, 2835), (1000 + 15√3, 2805)].

**Side wedge** π/3. Side film, anterior.

- The top normal is rotY(30°)ẑ = (½, 0, √3/2); its width axis stays ŷ and its
  depth axis is n × ŷ = (−√3/2, 0, ½), so the anterior end is 5 mm below
  the centre at z = 110. The bottom normal (−½, 0, √3/2) gives the depth
  axis (−√3/2, 0, −½), so its anterior end is 5 mm above z = 90.
- supA = (5√3, 0, 105) → (1000 + 10√3, 2790);
  supP = (−5√3, 0, 115) → (1000 − 10√3, 2770);
  infA = (5√3, 0, 95) → (1000 + 10√3, 2810);
  infP = (−5√3, 0, 85) → (1000 − 10√3, 2830).
  The front is 10 mm tall, the back 30 mm.

### `off-axis.json`

The vertebra of `tilted-and-wedged.json` given the same twist, rotZ(30°),
on two more films, and then with a deeper lower endplate. The endplates
stay level, at z = 110 and z = 90. The width axis ŷ turns to
(−½, √3/2, 0) and the depth axis −x̂ (front to back) to (−√3/2, −½, 0).
Every case puts every corner off the ends of the axes.

**Side film, anterior.** Along x the turned ellipse reaches
`√((15 · ½)² + (10 · √3/2)²) = √(56.25 + 75) = √131.25` mm, where the
ends of the axes reach only 7.5 and 8.66 mm. So supA and infA are at
u = 1000 + 2√131.25 = 1000 + √525, supP and infP at u = 1000 − √525,
with v = 2780 on top and 2820 below. `√525 = 22.9128784747792…`.

**Front film, patient left, one clockwise quarter turn.** Stored 4000 by
2000 with pixels 0.4 mm along a row and 0.5 mm down a column, as in
`turned-films.json`, so the upright pixel is `su = 0.5` wide and
`sv = 0.4` tall, and the stored point is `x = v`, `y = 1999 − u`.

- Across (y) the ellipse reaches √193.75 mm, as in the twist of
  `tilted-and-wedged.json`, so upright u = 1000 ± √193.75 / 0.5 =
  1000 ± √775. The top is at v = 3000 − 110 / 0.4 = 2725, the bottom at
  v = 3000 − 90 / 0.4 = 2775.
- supL (2725, 999 − √775), supR (2725, 999 + √775), infL
  (2775, 999 − √775), infR (2775, 999 + √775).
  `√775 = 27.83882181415011…`.
- Pedicle offsets turn to y′ = −6.25 ± 5.85√3, z = 103.6: upright
  u = 1000 + 2y′ = 987.5 ± 11.7√3, v = 3000 − 103.6 / 0.4 = 2741. So
  pedL (2741, 1011.5 − 11.7√3), pedR (2741, 1011.5 + 11.7√3).

**Front film, patient left, lower endplate 12 mm deep.** The top
endplate is as in the twist of `tilted-and-wedged.json`; only the lower
half-depth changes, from 10 to 12 mm. This is the one case that tells
the mean depth from one endplate's depth: a pedicle sits `1.25 d̄`
behind the centre, which only a twist turns across a front film, and
only here is `d̄` the mean of two different depths.

- The top endplate reaches √193.75 mm across, so supL and supR are at
  u = 1000 ± √775, v = 2780. The lower one reaches
  `√((15 cos 30°)² + (12 sin 30°)²) = √(168.75 + 36) = √204.75` mm, so
  infL and infR are at u = 1000 ± 2√204.75 = 1000 ± √819, v = 2820.
  `√819 = 28.61817604250837…`.
- The pedicles use d̄ = (10 + 12)/2 = 11 and w̄ = 15: offsets
  (−13.75, ±11.7, 3.6), which turn to y′ = −13.75 · ½ ± 11.7 · √3/2 =
  −6.875 ± 5.85√3. So pedL u = 986.25 + 11.7√3, pedR u =
  986.25 − 11.7√3, both at v = 2792.8. Taking the top depth alone
  (d̄ = 10) would put them 1.25 px to the right, at 987.5 ± 11.7√3.
