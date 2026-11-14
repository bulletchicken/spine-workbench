# spine-readings

Angles and curves on a fitted spine. `measure_spine(&ReadingsInput)`
returns a `SpineReadings`, with the fitted value and its uncertainty band
kept separate.

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
- Curves run between alternating extremes of the fitted plate tilts. Turns
  come from upper-plate tilts; each end uses the most tilted plate of its
  own kind in the neighbourhood bounded halfway to the adjacent turns.
  Equal-tilt plateaux use their first level. No minimum size is applied in
  the per-version Rust API used by level rules. The contract output keeps
  only nominal curves whose absolute Cobb reaches the evidence floor.
  The apex has the greatest sideways deviation from the chord between the
  two end centres. Apex levels T3–T5, T6–T11 and T12–L4 give the three
  regions. Curves outside those regions remain in the Rust geometry API
  but have no contract region. Contract curves are sorted by decreasing
  absolute Cobb.
- Each curve's Cobb is the signed front pair on its nominal end vertebrae.
  Apex twist uses its nominal apex. Counts of upper ends, lower ends and
  apices come from the largest curve of that region and convex side in
  each version. `versionsFound` separately counts versions in which the
  nominal curve’s fixed end pair still reaches the floor.
- Bending Cobb uses the standing curve's fixed end vertebrae and the
  smaller absolute residual from both marked bending films, with the
  standing curve's convexity sign in the contract. A single bend cannot
  establish the smaller residual. This reader holds bending marks fixed,
  so their band is a singleton; their stated spread is not propagated.
- The centre line through the front sacrum top and bending disc angles
  remain available through the Rust API. Disc angle is upper-bottom minus
  next-lower-top front tilt, positive opening to the patient's left.

For a curve, `valueDeg` is the Cobb angle on the spine as fitted between
its two end vertebrae, the most tilted plates of the fitted spine in their
turning neighbourhoods. Every other fitted reading uses its named levels;
bending Cobb uses the marked bending lines at the standing curve's fixed
ends. The band holds the 10th, 50th and 90th percentiles.
For every front-plane angle and side-plane plate angle these are the
closed-form normal percentiles: value ± 1.2815515655 times the angle's
standard deviation. A pair's variance is the sum of its two independent
plate variances. A front corner tilt uses the mean of its two plate
perturbations; without a corner tilt it uses the upper plate alone.
These bands do not change with the versions seed or count.

Forward lean and measured twist use the versions' percentiles, with linear
interpolation at p × (N−1). Forward lean follows the independently moved
side plates; twist uses the fit's reported marginal spread. Bending Cobb
keeps its singleton band; bending-mark uncertainty is not propagated.
With 32 versions a forward-lean band's half-width has a coefficient of
variation of about 15% across seeds (on the exact right-thoracic two-film
fit, T9 half-width was 1.75–4.45 degrees over 100 seeds). Its coverage below
uses one versions seed and count.

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
Upper and lower plates move independently in each measured plane. The
body rotation and wedges encode those two plate tilts; sizes remain fixed.
The shared translation of a body's marks changes no plate direction.

## What the band means

For front vertebral tilts and front pair angles, the band holds the angle
that exact marks would give about eight times in ten (79–81%), for the
spread the marks were given. For the main Cobb it does so about three
times in four on the two made-up thoracic curves (73% and 76%) and about
eight times in ten on the lumbar one (84%), likely because the most tilted
plates are picked by the same noise that is measured. A second independent marking falls inside
the band less often: an eight-in-ten band gives about six times in ten
(63.5%) under the independent normal reference. This is a normal-model
calculation, not a measured second-marking figure; main-Cobb second-marking
coverage has not been measured. Display
`valueDeg` plus or minus the larger of `p90 − valueDeg` and
`valueDeg − p10`, rounded up to a whole degree. Typical values stay labelled
as typical.

The band does not cover which vertebrae are the ends. The counts in
`upperEndSeen` and `lowerEndSeen` describe that separately. Ends stay fixed
for the Cobb band, while versions choose their own most tilted ends for
the endpoint and apex counts. `versionsFound` counts the fixed pair
reaching the 10 degree floor.

The spread rests on **the spread supplied with the marks, or the fit's
reported angle spread when those marks are unavailable**. With film marks
supplied, a plate's tilt SD is the stated own corner spread times
sqrt(k_a² + k_b²), divided by the distance between its corners: each k is
3 for an inferred corner and 1 otherwise, matching the fit's weights.
The same formula applies to the sacrum line. A millimetre spread uses
the film's stated spacing when it has one, and for a side film without one
the placement spacing with the side scale removed. A front film without a
stated spacing uses its placement spacing, which differs from the spacing
the fit weighted its marks with when only the side film states a scale.
A share of body
height is multiplied by the distance between the marked endplate midpoints.
Film pixels are made upright, with unequal pixel spacings retained.
Shared body shifts cancel from tilt. When a film or a level's marks are
unavailable, or a plate's two corners coincide, that plate uses sqrt(2)
times the vertebra's reported tilt SD (forward-lean SD for the side plane)
and the other plate keeps its mark-based spread. A marked body of zero
height gives both its plates that fallback. `fit_from_films` propagates its stated
share-of-height or millimetre spread; `fit_spine` estimates its reported
SDs from residuals. A fit alone does not say which of those uncertainty
models produced it.

The fit's body-rotation SD is not the same as the mark-line SD. On three
exact made-up spines at the stated front finder spread, sqrt(2) times the
fit SD was 0.810–0.995 times the plate-line SD. Film marks therefore take
precedence when available; the fit reports no plate covariance.
Without film marks the bands are narrower. The version ensembles used
by placement rules and the load bench use this fit-only fallback.

L1–S1 includes an independently uncertain sacrum line when the side film
states its spread in millimetres. A share of body height cannot supply a
sacrum spread because the sacrum has no marked body height: that band's
sacrum line is held fixed and its coverage is unmeasured. A sacrum line
with coincident ends gives L1–S1 `no-sacrum`, as do missing sacrum marks;
the other spine readings remain available.

Coverage checks use three made-up spines with independent Gaussian mark
noise at the stated spread, against readings of exact-mark fits. On noisy
marks the main thoracic angles have a measured bias of +3.1 to +3.9 degrees
at the trained finder's stated noise: the most tilted plates are picked
by the same noise that is then measured. Noise can also split one real
curve into shorter ones. The fixed-end band does not include choices of
ends or curve splitting. Real finder errors may not be independent or
Gaussian. These checks do not establish coverage on real films or on
minor curves.

## Measured coverage

The detailed measurements below use 300 fresh markings of each of three
made-up spines, recipe seed 4, mark seeds 120000–120299. Front corners have
independent Gaussian noise of 0.11 of body height and a shared body shift
of 0.08, with no pedicle marks. The full side film has independent 1 mm
corner noise and an independently marked sacrum line. Truth is each
exact-mark fit, rather than the recipe’s unfitted answer key. The written
half-width is unrounded. All angles below are degrees. Mean errors are
signed measured minus exact; main Cobb uses magnitudes.

| Main Cobb, front film alone | Written coverage | Mean magnitude error | Mean half-width |
|---|---|---|---|
| Right thoracic 40 | 213/300 (71.0%) | +3.47° | 9.56° |
| Double major 55/45 | 223/300 (74.3%) | +3.93° | 9.53° |
| Left lumbar 30 | 253/299 (84.6%) | +2.00° | 8.86° |

This 300-seed block is one draw, not a bound on bias or coverage. Its
right-thoracic coverage is 71.0%, below the coverage test’s 72% lower
bound; the coverage test uses 900 consecutive seeds per spine starting
at the same seed to reduce block variability. It asserts 72–90% coverage
and positive mean error on the two thoracic spines, with no upper bias
bound. The 900-seed run (mark seeds 120000–120899, including that block)
measured:

| Main Cobb, front film alone, 900 seeds | Written coverage | Mean magnitude error | Mean half-width |
|---|---|---|---|
| Right thoracic 40 | 659/900 (73.2%) | +3.14° | 9.61° |
| Double major 55/45 | 680/900 (75.6%) | +3.75° | 9.56° |
| Left lumbar 30 | 753/899 (83.8%) | +2.03° | 8.81° |

Six fresh blocks of 300 front-only markings per spine pooled 1370/1800
(76.1%), 1390/1800 (77.2%) and 1510/1800 (83.9%) coverage respectively.
The block ranges were 72.3–79.0%, 73.0–81.3% and 82.7–86.0%; the 900-seed
right-thoracic block reads low beside those fresh blocks. These are
empirical figures on those seeds, not universal coverage guarantees.

One left-lumbar marking had no main curve with the exact curve’s region
and side in both the 300-seed and 900-seed runs. Matching spans by
intersection/union at least one half, with the same convex side, finds
the main curve in 257/300, 289/300 and 285/300
front-only spines. Curves matching no exact curve average 1.710, 1.037
and 2.103 per spine respectively: noise can split a real curve.

The side film was used in 273/300, 280/300 and 272/300 fits respectively.
The following counts pool levels or pairs over the three spines; readings
within one spine are not independent trials. Forward lean uses 32 versions
and versions seed 7; its band varies with the versions seed.

| Reading | Written coverage | Mean error | Mean half-width |
|---|---|---|---|
| Front vertebral tilt, front film alone | 12107/15300 (79.1%) | +0.08° | 4.59° |
| Front pairs, front film alone | 98423/122400 (80.4%) | -0.08° | 9.26° |
| Main Cobb, front film alone | 689/899 (76.6%) | +3.14° | 9.32° |
| Front vertebral tilt, two-film request | 12107/15300 (79.1%) | +0.08° | 4.59° |
| Front pairs, two-film request | 99244/122400 (81.1%) | -0.08° | 9.26° |
| Main Cobb, two-film request | 696/899 (77.4%) | +3.14° | 9.33° |
| Forward lean | 11180/14025 (79.7%) | +0.01° | 3.25° |
| Side pairs | 83272/112200 (74.2%) | +0.12° | 6.46° |
| Kyphosis T2–T5 | 647/825 (78.4%) | +0.33° | 8.80° |
| Kyphosis T5–T12 | 609/825 (73.8%) | +0.39° | 6.38° |
| Kyphosis T10–L2 | 625/825 (75.8%) | -0.11° | 5.13° |
| Lordosis L1–L5 | 537/825 (65.1%) | +1.27° | 4.73° |
| Lordosis L1–S1 | 585/825 (70.9%) | +0.46° | 3.95° |
| Twist and apical twist | Not measured here | — | — |
| Bending Cobb | Not measured here | — | — |

The kyphoses and both lordoses are below eight times in ten. The
side-plane shortfall is thought to come mostly from front-film mark error
entering the fitted side-plane plate tilts, which these bands, built from
the side film’s spread alone, do not include: with a third of the front
noise and the same side noise, an earlier measurement had the side
readings hold 76–83% of the time. The coupling inside the fit and the
cause of the lower L5 plate’s upward bias have not been identified. The bands apply no correction multiplier.

The fit-only fallback on these same front-only noisy fits gives narrower
main-Cobb bands:

| Spine | Fit-only coverage | Mean error | Mean half-width | Width / mark-based width |
|---|---|---|---|---|
| Right thoracic 40 | 191/300 (63.7%) | +3.47° | 8.22° | 0.860 |
| Double major 55/45 | 193/300 (64.3%) | +3.93° | 8.21° | 0.861 |
| Left lumbar 30 | 243/299 (81.3%) | +2.00° | 7.80° | 0.881 |

These fallback figures use the most tilted ends. They describe the spread
model used by the placement-rule and load-bench version ensembles; those
ensembles do not take the mark-based plate spreads. Minor-curve coverage,
share-of-height sacrum uncertainty and real finder errors were not tested
for coverage here.

## Rust API and doors

- `readings::{NamedSampler, inverse_normal, version, versions}` retains
  dimension-name, seed, total count and global index identity in any batch.
  `Version` carries the fit and shared-child/per-pedicle draws. Its fit is
  unmodified by measuring it. `versions_for_readings` uses supplied film
  spreads; the fit-only functions use the reported-SD fallback.
- `curves::measure(&ReadingsInput, &SpineFit)` measures a nominal or sampled
  fit without classification. `curves::geometry` supplies
  coronal and sagittal tilts, regional kyphosis, centre line and bending
  measurements. Turns and ends are extremes of the fitted plate tilts.
  Missing observations use `ReadReason`.
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
