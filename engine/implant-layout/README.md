# implant-layout

The layout proposer takes a fitted spine and proposes a first placement:
upper and lower instrumented levels, a pattern of screws or hooks, each
screw's entry, direction, diameter and length, and two rods. It evaluates
published level rules together over repeatable versions of the same
spine, keeps their disagreements, and returns alternatives one change
away. A surgeon decides. This is a planning preview for education and
research, not a medical device and not clinical advice.

The engine returns numbers, provenance and reason codes. Wording for a
screen belongs to the app. It runs locally, uses embedded evidence, and
opens no files or network connections.

## Calling it

```rust,ignore
use implant_layout::propose_layouts;
use shapes::implant_layout::{ChildFacts, LayoutInput, NamingDoubt};

let input = LayoutInput {
    fit,
    front: None,
    bend_left: None,
    bend_right: None,
    child: ChildFacts {
        age_years: Some(14.0),
        weight_kg: None,
        activity: None,
    },
    naming: NamingDoubt::AsRead,
    versions: 32,
    seed: 1,
    pedicle_child_share: None,
};
let proposals = propose_layouts(&input)?;
let first = proposals.layouts.first();
```

`fit` is a `shapes::spine_fit::SpineFit`, including one produced by
`joint_fit::fit_from_films` with the front film alone and the built-in
typical spine. Supply a `LayoutFilm` in `front` to let rules read a marked
sacrum top. Supply both bending films for bending criteria. Their marks,
geometry and stored pixel sizes travel together. A missing film remains
`None`; another observation is never substituted for it. Weight and
activity are passed onward as child facts; they do not choose levels or
table screw sizes.

`LayoutProposals` contains curve and classification counts, selective
criteria, every rule's votes and unread counts, candidate level pairs,
layouts, a sourced size/material catalogue, cannot-tell codes and naming
scenarios. `layouts[0]` has id and change `first`. Anchors include every
pedicle within the instrumented span, top to bottom and left before
right, including empty pedicles (`kind: none`). Hooks and empty pedicles
have no screw diameter or length. Every absent optional field serialises
as `null`.

The contract also carries `firstIsSketch`, separate `upperEnds` and
`lowerEnds` counts, each rod's `standingPointsMm`, and per-anchor
`widerThanPedicle` counts and nullable `screwHere`. `DiameterCount` holds
`diameterMm` and `versions`. `ScrewHere` holds `diameterMm`,
`diameterAlsoSeen`, `lengthMm`, `lengthAlsoSeen`, nullable
`screwToPedicle` and nullable `medialAngle`: the screw size before a
pattern turns a pedicle into a hook or an empty place. On a hook or an
empty place, `diameterSources` and `lengthSources` are retained as the
sources behind `screwHere`'s table sizes. Pedicle `sources` cover the
width draws and the thresholds behind both width counts. Anchor
`diameterSources`, `lengthSources`, `directionSources` and
`lookSources` retain evidence per quantity; each `LookSources` pairs a
look `code` with its `sources`, beside the anchor's union of keys.
`firstIsSketch` marks a fallback span from the main curve when the rules
cannot supply a pair. Its end counts are empty; other end counts use the
same modal pair in each version, most frequent first. A sketch raises no
`ends-unsettled`. For a rule-derived pair, the pair's holding test still
raises `ends-unsettled`, judging each end by the level the first placement
uses and naming only unsettled ends unless both ends separately hold
steady. The standing rod line uses the target line's sampling stations,
so each occupied head has a corresponding knot
on each line. `geometryStatus` covers both lines; a typical line must
never be shown as measured millimetres.

`rebuild_layout(&RebuildInput) -> Result<ImplantLayout, LayoutError>` is
the entry for a placement changed by hand. Its input carries the original
`layout: LayoutInput`, `upper`, `lower`, `anchors`, `rodMaterial` and
`rodDiameterMm`. Each `AnchorChoice` has `level`, `side`, `kind`, nullable
`diameterMm` and nullable `lengthMm`; null asks for the size in
`screwHere`. Unspecified pedicles follow the first placement's pattern
over the new span, including table-driven substitutions. Rebuilding uses
the original seed and version identities and returns `changed-by-hand`
as its id and change. A changed screw size is `entered`, with an empty
source list and also-seen list; its width counts still describe the same
population draws. Its `screwHere` always keeps the size worked out from
the tables before the pattern, or stays null when no table size exists.
Unchanged sizes retain their table status, sources and also-seen lists.
Unfitted or crossed ends, spans shorter than two levels or outside the
fitted anatomical span from T1 through L5, duplicate or out-of-span
pedicles, sizes outside the grids, sizes on a hook or empty choice,
unsupported rod pairs, missing screw sizes, missing screw or hook
direction, and fewer than two occupied anchors on either side return
`case-invalid-value`. Other unavailable geometry retains its specific
refusal rather than being filled in.
Rod choices are sourced material and diameter pairs: not every material
and diameter in the two separate catalogue lists form a pair.

Errors distinguish invalid input, an empty fit and a refusal with a short
reason. They cross the bridge as existing engine error codes. The layout
proposer does not save a placement with the case.

`proposal::propose` is the assembly entry behind `propose_layouts`.
`proposal::measure(&input)` prepares the complete `LevelReadings`
ensemble. It registers an unscaled front film in the fit's drawn frame
using the fit's existing typical spacing. This provisional registration
allows the shared CSVL geometry and flip calculations to run; it does
not establish a physical film scale. `proposal::assemble(&input,
&level_readings)` applies the same input registration and accepts that
complete ordered ensemble. Its version indices must be exactly zero
through `versions - 1`; retain their global identities when assembling
draws computed in batches.

If no curve can supply a span, or the fitted profile lacks T5 or T12 for
the rod target, the readings still return with no layout and
`no-level-meets-the-rule`. A caller must check that `layouts` is nonempty.

## Coordinates, signs and provenance

The right-handed patient frame has x anterior, y to the patient's left
and z cranial. Its origin is the centre of L5's lower endplate, or the
fit's declared origin level when L5 is absent. Positions and contour
lengths use millimetres in this frame. Film marks stay in original film
pixels: x right, y down, with (0, 0) at the first pixel's centre. A film's
facing determines the mapping to patient left and right.

Fit angles and their SDs are radians, composed as
`rotX(side_tilt) * rotY(forward_lean) * rotZ(twist)`. Output angles are
degrees. Positive coronal tilt raises the patient's left side; positive
twist turns the body's front left. Signed Cobb is lower-bottom minus
upper-top front tilt, positive for a right-convex curve. Positive kyphosis
is upper-top minus lower-bottom sagittal tilt. Centre-line offset is
centre y minus sacral centre-line y, positive left. Disc angle is
upper-bottom minus next-lower-top front tilt, positive opening left.
Posterior positions have smaller x.

`measured`, `from-tables`, `from-template`, `entered`, `typical` and
`missing` are distinct statuses. A table value describes a population;
it is never a measurement of this pedicle. Published quantities carry
source keys resolved by the embedded `evidence/sources.json` registry.
The [evidence catalogue](evidence/README.md) explains the strict reader,
printed-cell qualifications, excluded cells and complete bibliography.
The rule table below identifies public sources directly; all citations
refer to that registry's recorded reading, which may be abstract only.

The frozen `LayoutCurve`, `CurveTypeCount` and `SelectiveCriterion`
shapes do not expose a status and source on every number. Their
provenance is partly implicit: curve geometry comes from film-derived
readings, classification applies the cited rules to those readings, and
selective criteria retain sources through their enclosing/sibling
fields. The literal requirement for per-number wire provenance remains
unmet; adding the missing fields belongs to a contract change. Rod
positions have a separate `geometryStatus` as described below.

`tier` is `front-film-only` exactly when `fit.sideFilm` is null, and
`two-films` otherwise. A supplied side film that was set aside by the fit
does not change the tier. With one front film, forward lean, depth, side
wedge and unmeasured twist belong to the typical spine. They never cast
measured votes. Sagittal criteria return `no-side-film`; unmeasured twist
returns `twist-not-measured`. Front coronal tilt uses `cornerTiltRad`
where the film supplies it, with projected endplate geometry otherwise.

When `fit.lengths` is `typical-size`, angles still have meaning. The
required rule is that a comparison against published millimetres or a
table pedicle width must be unreadable with `no-film-scale`: this covers
the deviated-vertebra rule, substantially-touched rule, lumbar modifier
and stable-minus-one's modifier dependency. These guards check
`FitLengths` even when the marks have provisional spacing in the drawn
frame. That registration does not establish a physical film scale.

Comparing two distances on the drawn spine, such as body contact or
pedicle-centre bracketing, does not need scale. Table screw sizes remain
available; anchor positions are `typical`. Rod points and lengths also
describe that typical drawing and must not be displayed as measured
millimetres. Each rod carries `geometryStatus`, filled by
`rods::geometry_status(&fit)`, separately from `targetKyphosisStatus`.

## Versions and classification

The shared sampler now lives in `spine-readings`; `readings` re-exports
its public sampler and adapts its errors. Every draw depends on seed,
dimension name, total version count and global index. Splitting a run
into batches leaves the versions unchanged. Combine a complete ensemble
before reporting counts. Bands use linear interpolation at
`p * (N - 1)` and give p10, p50 and p90. The same input produces the same
ordered output and JSON. Numbering doubt is a scenario rather than a
sampled probability.

Curves use alternating endplate tilt extremes. Their apex is the largest
sideways displacement from the chord between end centres [own
inference]. Classification uses the largest curve and structural minor
curves. A minor is structural at bending Cobb at least 25 degrees, or at
regional kyphosis at least 20 degrees (T2–T5 proximally, T10–L2 below).
One positive criterion establishes structural status; an absent test
never counts as a met or failed test. T5–T12 sagittal modifiers use less
than 10, 10 through 40, and greater than 40 degrees. Lumbar A/B/C compares
the sacral centre line with the apex's finite pedicle outlines and body;
version-specific table widths supply the outlines for placement rules.
Sources: Lenke et al. 2001, PMID 11507125 [peer-reviewed, abstract], and
AO Surgery Reference's Lenke classification [guideline, secondary].

The shared geometry API retains small curves for rule evaluation. The
`spine-readings` display contract separately applies its sourced
10-degree floor; that floor is not a licence to discard a small curve
needed by a placement rule. Bending readings use the same end vertebrae
as the standing curve and the smaller residual from both bends [own
inference]. Bending marks have no supplied covariance, so no mark-noise
spread is invented.

Placement curve bands reselect the largest curve of each matching region
and convex side in every version, so their end vertebrae can change.
The shared `spine-readings` display bands instead measure the nominal
curve's fixed end vertebrae in every version. These bands describe
different quantities: the placement band combines curve/end selection
with geometric variation, whereas the display band keeps endpoint
uncertainty separate. They must not be presented as interchangeable
measurements.

## Level rules and the first pair

Each rule supplies level counts, unread counts, its inputs and source
keys. Nominal flips express one input change in that input's own SD and
are replayed through the rule [own inference]. Unread rules do not vote.
An unavailable degree-to-Nash–Moe conversion stays unavailable.

| Rule | Computation and limits | Public source |
|---|---|---|
| `end-plus-one` | Lower end vertebra plus one anatomical level. | Liu et al. 2020, PMID 33401869, DOI 10.14245/ns.2040234.117 [peer-reviewed, full text]. |
| `suk-neutral` | Caudal measured twist zero/crossing supplies the neutral vertebra (NV); NV at most one below the lower end gives NV, otherwise NV minus one when at least two below. The nearer-zero crossing member and caudal tie, search beyond the curve end and use of a zero crossing are project inferences. | Suk 2003 as reported in Liu et al. 2020, PMID 33401869 [peer-reviewed, secondary in full text]. |
| `last-touched` | First body touched by the sacral centre line going down through T12–L5. Contact uses the body's own projected silhouette. | Beauchamp/Lenke 2020 as reported in Liu et al. 2020, PMID 33401869 [peer-reviewed, secondary in full text]. |
| `stable-minus-one` | First vertebra at/below the lower end whose pedicle centres bracket the centre line; A/B gives one above, C gives that vertebra. Bracketing is this project's reading of closely bisected. | Chen et al. 2026, PMID 41492130, DOI 10.1186/s12893-025-03481-9 [peer-reviewed, review full text, secondary rule]. |
| `substantially-touched` | An LTV with only non-substantial contact gives LTV plus one; otherwise the most proximal substantial pedicle contact supplies the lower end. Finite table-derived outlines are required. | Qin 2016 as reported in Liu et al. 2020, PMID 33401869 [peer-reviewed, secondary in full text]. |
| `deviated-vertebra` | Starting at the sacrum and going up, select the first vertebra more than 10 mm from the centre line. This proposes a lower end, rather than checking the next vertebra. | Wang 2011 as printed by Liu et al. 2020, PMID 33401869 [peer-reviewed, secondary in full text]. |
| `lenke-l3-l4` | Retains the apex-at/above-L1–L2 and convex L3–L4 disc tests, but the complete vote is unreadable with `no-rotation-grade`. | Chen et al. 2026, PMID 41492130, DOI 10.1186/s12893-025-03481-9 [peer-reviewed, review full text, secondary rule]. |
| `lev-plus-one-5c` | Type 5C gives LEV plus one if Cobb exceeds 60 degrees or the LEV/next disc does not reach neutral between the two bends; otherwise LEV. Reaching zero between bends includes an overshoot [own inference]. | Chen et al. 2026, PMID 41492130 [peer-reviewed, review full text, secondary rule]. |
| `upper-end-vertebra` | Highest included curve's upper end, with structural tests kept explicit. | Lenke et al. 2001, PMID 11507125 [peer-reviewed, abstract]; AO Surgery Reference [guideline]. |
| `uev-5c` | Type 5C votes for UEV; the posterior option UEV minus one remains a candidate without a second vote. | Chen et al. 2026, PMID 41492130 [peer-reviewed, review full text, secondary rule]. |
| `lenke-shoulder` | Always `shoulders-outside-the-model`. T1 tilt remains a labelled proxy input, never a shoulder vote. | Chen et al. 2026, PMID 41492130; Brooks et al. 2018, PMID 30348348, DOI 10.1016/j.jspd.2018.03.010; Wei Chan et al. 2025, PMID 40020691, DOI 10.1177/21925682251325171 [peer-reviewed; review full text, other two abstracts]. |
| `liv-index` | Pair check `(LIV-NV)+(LIV-SV) < 0`. It does not propose an end or express an individual outcome probability. | Chen et al. 2026, registry key `chen-2025`, PMID 40820306, DOI 10.1177/21925682251366988 [peer-reviewed, full text; 112-patient derivation cohort]. |

Selective thoracic criteria use MT:TL/L Cobb, apical translation and
rotation ratios greater than 1.2, TL/L bending below 25 degrees, T10–L2
kyphosis below 10 and TL/L Cobb below 60. Selective lumbar criteria use
TL/L:MT ratios greater than 1.25, thoracic bending below 25 and thoracic
Cobb below 40. These come through Kim et al. 2021 and Chen et al. 2026
(identifiers above, review full text). Applying degrees to a rotation
ratio and reading the review's printed TL/L bending row as thoracic
bending in the lumbar test are explicit project inferences. Unread
criteria are omitted from the conjunction and never counted as met;
having none readable leaves the conjunction unreadable. Both regions
are offered when the readable conjunction holds in 20–80% of versions
[own inference].

Candidate ends are evaluable rule modes and their fitted anatomical
neighbours. Crossed ends and ends outside T1–L5 are excluded. Pairs are
ordered by exact modal-rule agreements, then versions holding, then the
more caudal lower end; a remaining tie takes the more cranial upper end
[own inference]. The first placement uses the highest-ranked pair that
can actually form its anchors and both rods. Failed candidates are
removed and the next pair is tried. Holding below 60% gives
`ends-unsettled` [own inference]. Liu et al. 2020's stop-short odds
(2.60, 2.43, 3.05, 3.40 and 4.52 for the five landmarks) remain population
figures with their source and comparison, never personal probabilities.

Unreadable structural minors can leave no pair at all. In that case the
assembly starts a sketch from the nominal major curve's upper end
through its lower end plus one fitted anatomical level. It extends the
lower end sequentially, no farther than L5, until both rods can be formed
[own inference]. The sketch has zero versions holding and no invented
rule agreements; `ends-unsettled` remains explicit. It does not resolve
the unread minor curves. If none of these spans can be formed, no layout
is returned.

## Screws, patterns and rods

Widths are population draws, with one normal draw shared by the child
and one per pedicle. The default half-and-half variance split, positive
0.5 mm lower truncation, half-millimetre rounding uncertainty for
whole-millimetre cells, sex-column combination and interpolation of
side factors are project inferences. Thoracic ages 12 and above, or
unspecified, use Gao et al. 2017 controls (60 girls); younger thoracic
ages use Zheng et al. 2009's covered age bands (120 children). Lumbar
ages 12 and above use Upendra et al. 2010 (24 patients). Under-12 lumbar
widths remain missing. The pedicle estimator marks absent width bands `missing` and
keeps their look code. Candidate anchors are built lazily for needed
levels, so an unused distant trajectory cannot prevent a placement.
Sato et al. 2022's concave factors 0.73 at apex
plus/minus one and 1.08 near the neutral vertebra come from 57 adolescents
and 684 pedicles. When measured twist cannot supply a neutral vertebra,
this project's fallback uses the curve end on that side of the apex as
the neutral-zone edge. Overlapping curves use the nearer apex, with
top-to-bottom curve order breaking ties. Inner width subtracts Chiu et al. 2024's physical-side
and region-specific medial wall and 0.64 mm lateral wall; the wall
spreads are not sampled. Cohort body-width references and a population
adjustment are absent, so neither is invented.

Exact usable Upendra chord and angle cells take precedence over
interpolation. Only missing/excluded cells use neighbouring usable
levels [own inference]. The impossible printed L4 convex angle is
excluded. Zheng's full younger chord table includes ages 9–11, so no
nearest-age borrowing is needed there. Zheng's row ordering, divergent
negative angles and interpretation of its unlabelled ± as SD are project
readings. Gao's unlabelled ± is likewise read as SD by this project.
Sato's suspect neutral-width block remains excluded. Outside a curve,
this project's inference combines Upendra's concave and convex columns
with equal-weight means and the larger reported spread, retaining both
sources. This applies to lumbar widths and to chords and medial angles.
An absent curve-side label is not an absent age table; `no-table-for-this-age`
is reserved for genuinely uncovered size populations.

The straight-forward direction lies in the superior endplate's plane,
turned medially by the table angle. The table chord and fitted generic
body exit imply an entry point; no anatomical entry landmark is
located. Upendra is the default angle source; Zheng and Fennell/Peng
techniques are shown beside it because their axis definitions and
cohorts differ. Lehman et al. 2003's cadaver comparison supplies the
straight-forward basis. Directions include sampled angle spreads where
the tables print them.

The diameter rule chooses the largest grid size with no more than 10%
of versions below `diameter / 1.25`; 10% is a project threshold. Sato's
1.25 outer-width rule is shown beside Misenhimer's 0.80 and Upendra's
1.15. The diameter grid is inferred from reported sizes/ranges and the
smallest published grid step, carrying each covering source (Ramanathan
2026, Jiang 2022, Sarwahi 2023 and Grabala 2023). It is not a manufacturer's
complete catalogue. Ratio bands remain available without treating a
larger screw-to-pedicle ratio as an outcome estimate.

Lengths use the longest selected Balikci catalogue size whose cap holds
in at least 90% of versions [own inference]:
`min(chordShare * chord, chord - anteriorMargin)`. The age-12-and-above
share is 0.8; the younger 0.65 is this project's midpoint of Zheng's 0.5–0.8
range. The 4 mm anterior margin comes from Sarwahi et al. 2017, whose
aorta-contact experiment is a warning basis rather than a patient
boundary. Selected lengths are 25–45 mm in 2.5 mm steps, rounded down;
when none qualifies, the anchor retains the shortest catalogue length
(25 mm) and `shorter-than-tables`. Missing chord tables still leave the
length null. Usual regional lengths from Balikci
and Kuraishi are displayed for comparison and never clamp the result.

The first pattern occupies all qualifying pedicles. Missing qualifying
diameter, length or trajectory becomes a hook at the upper end and an
empty pedicle elsewhere [own inference]. `key-levels` prioritises ends,
apex and neighbours, then alternates sides subject to the 1.4-per-level
cap from Larson et al. 2024's MIMO trial (211 patients). Priority on a
short span is a project rule. Bilateral key pairs are never split to
fill an odd density-cap slot. Assembly applies alternating sides in a
canonical right-convex frame, swapping sides for
a left-convex major curve so the pattern reflects with that curve.
`hooks-at-top` substitutes upper hooks;
Zhao et al. 2024's association and Cammarata et al. 2014's numerical study
are evidence for offering an alternative, not proof of its outcome.

`releasedLevels` names the joint below each screw-bearing vertebra
inside the instrumented span, excluding the lower end's outside joint
[own inference]. It records where the bench applies its capsule-release
factor. Hooks alone do not cause that factor, and the input does not
measure an actual surgical release.

The first rods are cobalt chrome, 5.5 mm, supported by Wilke et al.
2024's 32-surgeon survey and Sia et al. 2019. Sourced alternatives include
cobalt chrome 6.0 mm (Wang et al. 2016) and titanium alloy 5.5 mm (Sia).
Abe et al. 2015 supplies titanium alloy 6.0 mm to the change catalogue.
Applying a published pair to both rods is a project choice; Wang used
the larger rod only on the concave side. Unsourced diameters and
stainless steel are not offered merely because an enum can name them.

The target is built by joint rotations alone [own inference]. The
lowest instrumented vertebra keeps its standing pose. Upward from it,
each relative coronal component becomes zero, axial components stay
standing, and the facing endplate midpoint stays fixed in both bodies.
Only instrumented T5/T6 through T11/T12 sagittal components share a
scale to reach the measured T5–T12 kyphosis clamped to 10–40 degrees.
T12/L1 and lumbar sagittal components remain standing. Without measured
side geometry the sagittal profile stays typical and unscaled. No
vertebral centre is moved onto a line. `coronalLine` is reference
metadata retained by the frozen contract; it does not move the target.

Rod contours interpolate the occupied heads on this target with a
clamped cubic or exact straight/circular construction, keeping every
zero-overbend head knot and samples at no more than 2 mm arc intervals
[own inference]. The 15 mm head offset is borrowed from Borrelli et al.
2025's lumbar cantilever model, not a measured adolescent head height.
Hooks hold at their entry. Both first rods have `overbendDeg: 0`.
Surgeons commonly bend a concave rod further to offset flattening
(Cidambi 2012, Salmingo 2014 and Sia 2019); this first placement cannot
show permanent yielding. The explicit overbend API is retained for a
later model. Proposal points and length are head to head. The bench's
20 mm straight tangent continuations during seating are a separate
project choice, not a published overhang measurement. Its holder fixes
rod twist about the supplied chord relative to the supplied contour;
after screw locking it removes that hold. Tightened screws tie all six
relative freedoms; hooks tie translations only. Head slip is absent.

Alternatives each change one thing from the first: either end by one
level, fewer screws, hooks at the top, or a stiffer/softer sourced rod
pair. Each is rebuilt and retained only with valid fitted uncrossed
ends in T1–L5. If naming may be one off, two separate scenarios shift
names by minus/plus one and recompute changes in anchor kind, size and
look codes. They are not assigned probabilities; the finder's offset
rate is unknown.

## What needs a look, and what cannot be known

Anchor look codes retain concave apex/adjacent pedicles at T3–T9
(`concave-apex-needs-ct`, Sato/Gao), too-small width/channel or no diameter
(`may-be-too-narrow`, project threshold with Akazawa's channel boundary),
left T4–T5 or T10–T12 in a right thoracic curve (`near-the-aorta`,
Takeshita), wide convergence or twist beyond 30 degrees
(`direction-unreadable`, Cerny's visibility limit), lengths below the
catalogue, uncovered table ages, and any nearest-band chord use. Mirror
aorta-warning levels for left curves are not supplied and are not
invented. Whole-spine codes retain absent sacrum, bends or side film,
unmeasured twist, unsettled ends, numbering doubt and rule-specific
missing observations. Bending acquisition posture is not in the input;
`standing-bends` conservatively records that the source rules used
supine bends. `may-be-too-narrow` uses only the pedicle's own evidence:
no grid diameter qualifies under the chosen rule, or the drawn outer
width is below the existing 3.2 mm boundary in more than 10% of
versions, or the inner channel is below 1 mm in more than 10%. The share
threshold is the diameter rule's existing project threshold. A secondary
diameter rule with no qualifying grid size remains in `diametersBeside`
with a null `diameterMm` and its source. Naming scenarios use the same
look triggers.
Missing values and inapplicable rules stay distinct. Dimensional rules
emit `no-film-scale` for a typical-size fit.

The limits that follow apply to interpreting these placements and to
passing them to the load bench.

1. This is an education and research preview; a surgeon decides.
2. Sizes come from age/level/curve-side tables, never this child's own
   measured pedicles. Reported width SDs of about 1–1.6 mm exceed the
   usual 0.5 mm diameter step. A CT would show the actual pedicle.
3. The generic bone shapes derive from one adult's vertebrae shaped to
   the fit. Entry points come from tables and generic geometry, not
   landmarks found on this bone.
4. Films without a known applied load cannot identify tissue stiffness.
   Absolute forces depend on assumed stiffness; paired placements are
   compared on the same versions. At typical size the bench's lever
   arms also belong to a typical-sized child.
5. The bench's rods can flex elastically but cannot bend permanently.
   Elastic-only correction can therefore be an upper end. Versions
   outside the stiff-rod method's range must remain counted; this crate
   does not calculate forces or rod yield.
6. Joint stiffness comes from adult cadavers and a published planner's
   table, with no measured child stiffness. Strengths come from adult
   bone and young animals. Table populations are mainly Chinese,
   Japanese, Thai and Indian; no population correction is applied.
7. Daily loads omit muscles and use adult telemetry multiples. Activities
   enlarge standing loads; they do not reconstruct a child's movement.
8. A year of walking compares rod swings with a laboratory test level;
   it gives neither a lifetime nor a date. Fusion time, relaxation,
   growth and maturity-to-remaining-growth are not modelled.
9. Fit noise is independent per vertebra because only marginal SDs are
   supplied. This can overstate adjacent joint-angle noise. Typical
   one-film sagittal and inferred twist values are not sampled as
   measurements.
10. Level names can be one off. Naming scenarios show consequences and
    carry no inferred probability. Shoulders, C7 plumb-line balance,
    missing sacrum geometry and an unavailable rotation-grade mapping
    cannot be recovered from the fitted column.
11. A placement is not saved with the case.
12. Published simulators' angular comparisons do not establish forces.
    This model has not been compared with patients. Its numerical and
    symmetry checks establish implementation identities, not clinical
    performance.
13. Outside the rods, the bench keeps the standing joint rest shape.
    Spontaneous correction of an unfused curve and body rebalancing are
    absent. Read shape results inside the instrumented levels only.
14. Tightened-head slip, actual adolescent head offset, a measured rod
    overhang, muscles, growth-friendly eligibility and the body's
    response to history or non-idiopathic disease are outside the
    supplied inputs. No history-based applicability claim is made.

## Public citations

The registry keeps full authors, titles, populations, denominators,
identifiers and reading qualifications. The compact list below includes
the public papers behind active placement rules and the retained
comparison evidence. `[full text]` and `[abstract]` report the supplied
reading status, rather than a new assessment of a paper. Publications
below are peer-reviewed except the explicitly marked conference
abstract and guideline.

- `gao-2017`: Gao B et al. 2017, [PMID 28766159](https://pubmed.ncbi.nlm.nih.gov/28766159/), [DOI 10.1007/s11999-017-5448-9](https://doi.org/10.1007/s11999-017-5448-9) [full text].
- `upendra-2010`: Upendra B et al. 2010, [PMID 20419004](https://pubmed.ncbi.nlm.nih.gov/20419004/), [DOI 10.4103/0019-5413.62084](https://doi.org/10.4103/0019-5413.62084) [full text].
- `pluemvitayaporn-2025`: Pluemvitayaporn T et al. 2025, [PMID 40041581](https://pubmed.ncbi.nlm.nih.gov/40041581/), [DOI 10.1055/s-0044-1800812](https://doi.org/10.1055/s-0044-1800812) [full text].
- `zheng-2009`: Zheng C et al. 2009, [PMID 18956181](https://pubmed.ncbi.nlm.nih.gov/18956181/), [DOI 10.1007/s00264-008-0675-z](https://doi.org/10.1007/s00264-008-0675-z) [full text].
- `chiu-2024`: Chiu CK et al. 2024, [PMID 38408520](https://pubmed.ncbi.nlm.nih.gov/38408520/), [DOI 10.1016/j.spinee.2024.02.014](https://doi.org/10.1016/j.spinee.2024.02.014) [abstract].
- `sato-2022`: Sato T et al. 2022, [PMID 36068555](https://pubmed.ncbi.nlm.nih.gov/36068555/), [DOI 10.1186/s12891-022-05799-4](https://doi.org/10.1186/s12891-022-05799-4) [full text].
- `akazawa-2015`: Akazawa T et al. 2015, [PMID 26124077](https://pubmed.ncbi.nlm.nih.gov/26124077/), [DOI 10.1007/s00776-015-0746-0](https://doi.org/10.1007/s00776-015-0746-0) [abstract].
- `misenhimer-1989`: Misenhimer GR et al. 1989, [PMID 2718038](https://pubmed.ncbi.nlm.nih.gov/2718038/), [DOI 10.1097/00007632-198904000-00004](https://doi.org/10.1097/00007632-198904000-00004) [abstract].
- `grabala-2023`: Grabala P et al. 2023, [PMID 37629410](https://pubmed.ncbi.nlm.nih.gov/37629410/), [DOI 10.3390/jcm12165368](https://doi.org/10.3390/jcm12165368) [full text].
- `sarwahi-2023`: Sarwahi V et al. 2023, [PMID 37134132](https://pubmed.ncbi.nlm.nih.gov/37134132/), [DOI 10.1097/brs.0000000000004700](https://doi.org/10.1097/brs.0000000000004700) [abstract].
- `katsumi-2025`: Katsumi S et al. 2025, [PMID 40856839](https://pubmed.ncbi.nlm.nih.gov/40856839/), [DOI 10.1007/s00586-025-09210-3](https://doi.org/10.1007/s00586-025-09210-3) [abstract].
- `jiang-2022`: Jiang J et al. 2022, [PMID 36333715](https://pubmed.ncbi.nlm.nih.gov/36333715/), [DOI 10.1186/s12893-022-01827-1](https://doi.org/10.1186/s12893-022-01827-1) [full text].
- `balikci-2019`: Balikci T et al. 2019, [PMID 31079434](https://pubmed.ncbi.nlm.nih.gov/31079434/), [DOI 10.31616/asj.2018.0177](https://doi.org/10.31616/asj.2018.0177) [full text].
- `kuraishi-2013`: Kuraishi S et al. 2013, [DOI 10.1186/1748-7161-8-S1-O27](https://doi.org/10.1186/1748-7161-8-S1-O27) [conference abstract; abstract].
- `sarwahi-2017`: Sarwahi V et al. 2017, [PMID 28296814](https://pubmed.ncbi.nlm.nih.gov/28296814/), [DOI 10.1097/brs.0000000000002153](https://doi.org/10.1097/brs.0000000000002153) [abstract].
- `takeshita-2010`: Takeshita K et al. 2010, [PMID 20127493](https://pubmed.ncbi.nlm.nih.gov/20127493/), [DOI 10.1007/s00586-010-1298-9](https://doi.org/10.1007/s00586-010-1298-9) [abstract].
- `fennell-2014`: Fennell VS et al. 2014, [PMID 25147977](https://pubmed.ncbi.nlm.nih.gov/25147977/), [DOI 10.3171/2014.7.spine1489](https://doi.org/10.3171/2014.7.spine1489) [abstract].
- `peng-2023`: Peng F et al. 2023, [PMID 37609000](https://pubmed.ncbi.nlm.nih.gov/37609000/), [DOI 10.3389/fsurg.2023.1219816](https://doi.org/10.3389/fsurg.2023.1219816) [full text].
- `lehman-2003`: Lehman RA et al. 2003, [PMID 14501914](https://pubmed.ncbi.nlm.nih.gov/14501914/), [DOI 10.1097/01.brs.0000087743.57439.4f](https://doi.org/10.1097/01.brs.0000087743.57439.4f) [abstract].
- `ramanathan-2026`: Ramanathan R et al. 2026, [PMID 42364200](https://pubmed.ncbi.nlm.nih.gov/42364200/), [DOI 10.5152/j.aott.2026.25525](https://doi.org/10.5152/j.aott.2026.25525) [full text].
- `ao-lenke`: AO Surgery Reference, year unspecified, [Lenke classification](https://stg.edit.surgeryreference.aofoundation.org/spine/deformities/adolescent-idiopathic-scoliosis/further-reading/lenke-classification) [guideline, secondary].
- `lenke-2001`: Lenke LG et al. 2001, [PMID 11507125](https://pubmed.ncbi.nlm.nih.gov/11507125/) [abstract].
- `liu-2020`: Liu CW et al. 2020, [PMID 33401869](https://pubmed.ncbi.nlm.nih.gov/33401869/), [DOI 10.14245/ns.2040234.117](https://doi.org/10.14245/ns.2040234.117) [full text].
- `chen-2025`: Chen IH et al. 2026, [PMID 40820306](https://pubmed.ncbi.nlm.nih.gov/40820306/), [DOI 10.1177/21925682251366988](https://doi.org/10.1177/21925682251366988) [full text].
- `bmc-level-review`: Chen R et al. 2026, [PMID 41492130](https://pubmed.ncbi.nlm.nih.gov/41492130/), [DOI 10.1186/s12893-025-03481-9](https://doi.org/10.1186/s12893-025-03481-9) [full text].
- `korean-level-review`: Kim DH et al. 2021, [PMID 34044493](https://pubmed.ncbi.nlm.nih.gov/34044493/), [DOI 10.3340/jkns.2020.0258](https://doi.org/10.3340/jkns.2020.0258) [full text].
- `larson-2013`: Larson AN et al. 2013, [PMID 27927354](https://pubmed.ncbi.nlm.nih.gov/27927354/), [DOI 10.1016/j.jspd.2013.05.009](https://doi.org/10.1016/j.jspd.2013.05.009) [abstract].
- `mimo-2024`: Larson AN et al. 2024, [PMID 37973031](https://pubmed.ncbi.nlm.nih.gov/37973031/), [DOI 10.2106/jbjs.23.00178](https://doi.org/10.2106/jbjs.23.00178) [abstract].
- `zhao-2024`: Zhao J et al. 2024, [PMID 39205666](https://pubmed.ncbi.nlm.nih.gov/39205666/), [DOI 10.3389/fped.2024.1387841](https://doi.org/10.3389/fped.2024.1387841) [full text].
- `cammarata-2014`: Cammarata M et al. 2014, [PMID 24480964](https://pubmed.ncbi.nlm.nih.gov/24480964/), [DOI 10.1097/brs.0000000000000222](https://doi.org/10.1097/brs.0000000000000222) [abstract].
- `wilke-2024`: Wilke HJ et al. 2024, [PMID 38633662](https://pubmed.ncbi.nlm.nih.gov/38633662/), [DOI 10.1002/jsp2.1324](https://doi.org/10.1002/jsp2.1324) [full text].
- `cidambi-2012`: Cidambi KR et al. 2012, [PMID 22426445](https://pubmed.ncbi.nlm.nih.gov/22426445/), [DOI 10.1097/brs.0b013e318252ccbe](https://doi.org/10.1097/brs.0b013e318252ccbe) [abstract].
- `borrelli-2025`: Borrelli S et al. 2025, [PMID 40082329](https://pubmed.ncbi.nlm.nih.gov/40082329/), [DOI 10.1007/s10439-025-03706-1](https://doi.org/10.1007/s10439-025-03706-1) [full text].
- `brooks-2018`: Brooks JT et al. 2018, [PMID 30348348](https://pubmed.ncbi.nlm.nih.gov/30348348/), [DOI 10.1016/j.jspd.2018.03.010](https://doi.org/10.1016/j.jspd.2018.03.010) [abstract].
- `wang-2016`: Wang X et al. 2016, [PMID 27750078](https://pubmed.ncbi.nlm.nih.gov/27750078/), [DOI 10.1016/j.clinbiomech.2016.10.002](https://doi.org/10.1016/j.clinbiomech.2016.10.002) [abstract].
- `abe-2015`: Abe Y et al. 2015, [PMID 25810754](https://pubmed.ncbi.nlm.nih.gov/25810754/), [DOI 10.1186/1748-7161-10-S2-S2](https://doi.org/10.1186/1748-7161-10-S2-S2) [full text].
- `uiv-tilt-40020691`: Wei Chan CY et al. 2025, [PMID 40020691](https://pubmed.ncbi.nlm.nih.gov/40020691/), [DOI 10.1177/21925682251325171](https://doi.org/10.1177/21925682251325171) [abstract].
- `salmingo-2014`: Salmingo RA et al. 2014, [PMID 24275616](https://pubmed.ncbi.nlm.nih.gov/24275616/), [DOI 10.1016/j.spinee.2013.08.042](https://doi.org/10.1016/j.spinee.2013.08.042) [abstract].
- `sia-2019`: Sia U et al. 2019, [PMID 31001378](https://pubmed.ncbi.nlm.nih.gov/31001378/), [DOI 10.5704/moj.1903.002](https://doi.org/10.5704/moj.1903.002) [full text].
- `cerny-2014`: Cerny P et al. 2014, [PMID 25120581](https://pubmed.ncbi.nlm.nih.gov/25120581/), [DOI 10.1186/1748-7161-9-11](https://doi.org/10.1186/1748-7161-9-11) [full text].
- `boissiere-2023`: Boissiere L et al. 2023, [PMID 38255344](https://pubmed.ncbi.nlm.nih.gov/38255344/), [DOI 10.3390/children11010029](https://doi.org/10.3390/children11010029) [abstract].
- `cui-2012`: Cui G et al. 2012, [PMID 21739245](https://pubmed.ncbi.nlm.nih.gov/21739245/), [DOI 10.1007/s00276-011-0849-z](https://doi.org/10.1007/s00276-011-0849-z) [abstract].
- `etemadifar-2017`: Etemadifar M et al. 2017, [PMID 28349022](https://pubmed.ncbi.nlm.nih.gov/28349022/), [DOI 10.4103/2277-9175.201331](https://doi.org/10.4103/2277-9175.201331) [full text].
- `ueno-2025`: Ueno J et al. 2025, [PMID 40299013](https://pubmed.ncbi.nlm.nih.gov/40299013/), [DOI 10.1007/s00586-025-08882-1](https://doi.org/10.1007/s00586-025-08882-1) [abstract].
- `s1-liv-42811138`: Ohrt-Nissen S et al. 2026, [PMID 42811138](https://pubmed.ncbi.nlm.nih.gov/42811138/), [DOI 10.1007/s00586-026-10407-3](https://doi.org/10.1007/s00586-026-10407-3) [abstract].
- `ssv-40103372`: Hori Y et al. 2025, [PMID 40103372](https://pubmed.ncbi.nlm.nih.gov/40103372/), [DOI 10.1097/brs.0000000000005336](https://doi.org/10.1097/brs.0000000000005336) [abstract].
- `djk-34096362`: Wang PY et al. 2023, [PMID 34096362](https://pubmed.ncbi.nlm.nih.gov/34096362/), [DOI 10.1177/21925682211019692](https://doi.org/10.1177/21925682211019692) [abstract].
- `l3-bending-30739071`: Qin X et al. 2019, [PMID 30739071](https://pubmed.ncbi.nlm.nih.gov/30739071/), [DOI 10.1016/j.clineuro.2019.02.005](https://doi.org/10.1016/j.clineuro.2019.02.005) [abstract].
- `residual-t1-39658742`: Banno T et al. 2025, [PMID 39658742](https://pubmed.ncbi.nlm.nih.gov/39658742/), [DOI 10.1007/s43390-024-01020-9](https://doi.org/10.1007/s43390-024-01020-9) [abstract].
- `shoulder-36138576`: Banno T et al. 2023, [PMID 36138576](https://pubmed.ncbi.nlm.nih.gov/36138576/), [DOI 10.31616/asj.2022.0020](https://doi.org/10.31616/asj.2022.0020) [abstract].
- `coronal-decompensation-4417731`: Studer D et al. 2015, [PMID 25845647](https://pubmed.ncbi.nlm.nih.gov/25845647/), [DOI 10.1007/s11832-015-0653-0](https://doi.org/10.1007/s11832-015-0653-0) [full text].
- `kuraishi-2013-full-paper`: Kuraishi S et al. 2013, [PMID 21959835](https://pubmed.ncbi.nlm.nih.gov/21959835/), [DOI 10.1097/BSD.0b013e31823162ef](https://doi.org/10.1097/BSD.0b013e31823162ef) [abstract].
