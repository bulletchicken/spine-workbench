# spine-readings

Angles and curves on a fitted spine. `measure_spine(&ReadingsInput)`
returns a `SpineReadings`, with the fitted value and the spread over
repeatable versions kept separate.

## Definitions and signs

The patient frame is x forward, y to the patient's left, z up. Angles in
this contract are degrees; the fit's angles and standard deviations are
radians. Fitted levels and pairs run top to bottom.

- Coronal tilt is `cornerTiltRad` when present: positive when the patient's
  left side is higher. Otherwise it is the upper endplate's projected
  normal, `atan2(-normal.y, normal.z)`. With a corner tilt the upper and
  lower front endplates take plus and minus half the fitted front wedge.
  This keeps a typical lean, side wedge or twist out of measured front
  angles. Without a corner tilt both plates use the forward model's
  projected normals.
- Forward lean and twist are the fit's named angles. Positive lean tips
  the body forward going up; positive twist turns its front to the left.
  An inferred twist stays not-measured with its fitted typical value.
- Every front pair is the lower vertebra's lower-endplate tilt minus the
  upper vertebra's upper-endplate tilt. Positive is right-convex. Every
  side pair is upper-top minus lower-bottom sagittal tilt, positive for
  kyphosis; sagittal normal tilt is `atan2(normal.x, normal.z)`.
- The three kyphoses use T2–T5, T5–T12 and T10–L2. Lordosis L1–L5 is the
  opposite sign of that side pair, so lordosis is positive. Lordosis
  L1–S1 is sacrum-top tilt minus L1 upper-endplate tilt. The sacrum line
  comes from `sacA` and `sacP` on the supplied side film, made upright and
  mapped through the fit's side-film spacing, origin and facing. Its
  upward-normal tilt is minus its anterior-to-posterior line slope.
- Curves run between alternating tilt extremes, with upper and lower
  plates searched in their own neighbouring extreme's interval. No
  minimum size is applied in the per-version Rust API used by level rules.
  The contract output keeps only nominal curves whose absolute Cobb reaches
  the evidence floor. The apex has the greatest sideways deviation from the chord between the two end centres. Apex levels T3–T5,
  T6–T11 and T12–L4 give the three regions. Curves outside those regions
  remain in the Rust geometry API but have no contract region. Contract
  curves are sorted by decreasing absolute Cobb.
- Each curve's Cobb is the signed front pair on its nominal end vertebrae.
  Apex twist uses its nominal apex. Counts of upper ends, lower ends and
  apices come from the largest curve of that region and convex side in
  each version. `versionsFound` separately counts versions in which the
  nominal curve’s fixed end pair still reaches the floor.
- Bending Cobb uses the standing curve's fixed end vertebrae and the
  smaller absolute residual from both marked bending films, with the
  standing curve's convexity sign in the contract. A single bend cannot
  establish the smaller residual. Bending marks have no fitted covariance,
  so their band is a singleton rather than invented mark noise.
- The centre line through the front sacrum top and bending disc angles
  remain available through the Rust API. Disc angle is upper-bottom minus
  next-lower-top front tilt, positive opening to the patient's left.

`valueDeg` always belongs to the spine as fitted. `band` is the 10th,
50th and 90th percentile of that same quantity on the versions, using
linear interpolation at p × (N−1). Pair bands are computed from paired
angles in each version, never by subtracting marginal bands.

## The contract curve floor

Only a curve whose nominal Cobb magnitude is at least 10 degrees enters
`SpineReadings.curves`. Smaller wobbles remain available through every
vertebra’s tilt, all pair angles and the uncut per-version Rust API. The
number is loaded by the strict embedded evidence reader from
`evidence/curve-floor.json`, with its unit and source key.

The source is Negrini S, Donzelli S, Aulisa AG, Czaprowski D, Schreiber S,
de Mauroy JC, Diers H, Grivas TB, Knott P, Kotwicki T, et al., *2016 SOSORT
guidelines: orthopaedic and rehabilitation treatment of idiopathic scoliosis
during growth*, Scoliosis Spinal Disord. 2018;13:3, PMID 29435499,
DOI [10.1186/s13013-017-0145-8](https://doi.org/10.1186/s13013-017-0145-8),
PMCID [PMC5795289](https://pmc.ncbi.nlm.nih.gov/articles/PMC5795289/).
The full text was read; the source is labelled guideline. Its SRS statement
combines the angular threshold with recognisable axial rotation. This
engine uses only the angular floor for naming output curves; it does not
turn that threshold into a diagnostic result.

## Two films and one

`fit.sideFilm` determines the tier. Two films allow forward lean, side
pairs, kyphosis and L1–L5 lordosis where the endpoint geometry is measured.
L1–S1 additionally needs the marked side sacrum; absent fitted endpoints
are `not-fitted`. With the front film alone these side readings are
`not-measured`, reason `no-side-film`, and carry the fitted typical value
where one exists. When the side film covers only some vertebrae, readings
that need an unseen vertebra use `not-on-side-film` and carry the fitted
typical value; the tier stays `two-films`. The fit contains no typical
sacrum top, so L1–S1 has `typicalDeg: null` without a side film.

| Reason | What is missing |
|---|---|
| `no-side-film` | a side film |
| `not-on-side-film` | the side film does not show these vertebrae |
| `twist-not-measured` | pedicle marks or an independent twist reading |
| `no-sacrum` | the sacrum top marked on the film |
| `no-bending-films` | both marked bending films |
| `no-curve` | a curve to read |
| `not-fitted` | the vertebra in the fit |

Coronal tilt, front pairs and curves remain measured in either tier.
Twist and apex twist are measured only where `inferred.twist` is false.
Bending Cobb needs both marked bends in either tier. Length scale changes
none of these angles: `typical-size` is allowed.

A not-measured reading keeps its reason and a separate nullable
`typicalDeg`. Typical values never enter measured readings. Versions leave
front-only depth position and lean, and inferred twist, undrawn: their
reported SDs describe typical variation rather than measurement error.
The front corner tilt moves only by its own fitted side-tilt SD. Sizes
and wedges remain fixed, as in the original version sampler.

## Two things a screen should know

The curve's band measures the **same end vertebrae in every version**.
Choosing the most tilted ends afresh selects extremes of noise and
inflates the angle. Endpoint uncertainty is given separately by
`upperEndSeen` and `lowerEndSeen`; it must not be folded into Cobb noise.

The fit supplies marginal SDs without neighbouring correlations. Versions
move measured vertebrae independently, so the band for adjacent-level
angles can be somewhat wider than their actual joint uncertainty.

To display a reading, use `valueDeg` plus or minus the larger of
`p90 − valueDeg` and `valueDeg − p10`, rounded up to a whole degree, and
say the band holds the middle eight in ten of the engine's versions.
Typical values must be labelled as typical, never measurements.

## Rust API and doors

- `readings::{NamedSampler, inverse_normal, version, versions}` retains
  dimension-name, seed, total count and global index identity in any batch.
  `Version` carries the fit and shared-child/per-pedicle draws. Its fit is
  unmodified by measuring it.
- `curves::measure(&ReadingsInput, &SpineFit)` measures a nominal or sampled
  fit without classification. `curves::geometry` supplies
  coronal and sagittal tilts, regional kyphosis, centre line and bending
  measurements. Missing observations use `ReadReason`.
- `bands::band` is the shared percentile convention.
- The bridge exports `measure_spine`; the spine worker's `measure-spine`
  job and `EngineClient.measureSpine` use the same contract.
- `cargo spine measure --recipe right-thoracic-40 --seed 1` makes exact
  made-up film marks, fits two films and prints a compact table. Add
  `--front-only` for the one-film fit, or `--json` for the contract JSON.
  `cargo spine measure --film front=PATH [--film side=PATH] [--json]`
  instead runs the trained front finder, the pass for a supplied side film,
  the built-in typical fit and these same readings. Trained marks use a
  spread of 0.11 of body height and a shared part of 0.08. A side film needs
  a stated scale; no side scale is invented. The compact header names the
  films used and whether lengths are measured or typical-size.

The engine is deterministic, uses the seeded streams and `libm`, and
opens no files or network connections. This is a geometry preview for
education and research; films do not identify tissue behaviour or a
patient's own bone shape.
