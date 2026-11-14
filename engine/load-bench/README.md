# load-bench

A planning preview for education and research. It estimates what a supplied
placement does in a passive spine model, repeats that calculation across
versions of the unknown geometry and tissue properties, and compares placements
on matching versions. It is not a medical device or clinical advice. A surgeon
decides.

The bench takes a fitted spine and placement proposals from `implant-layout`.
It seats the rods, locks the heads, lets go of the holders, allows one elastic
springback step, and adds daily load cases. It reports ranges, parts' relative
loads, counts past published strength and paired counts against the first
placement. It gives no growth forecast, implant lifetime or date.

## Calling it

`run_load_bench(&BenchInput) -> Result<BenchBatch, BenchError>` runs one slice of
the complete ensemble. Give it the fit, placements, child facts, seed, total
`versions`, `version_from` and `version_count`. The seed and complete version
count must be the ones used for the placement proposals. A four-version batch
is a useful worker scheduling unit; the caller chooses batch size.

Each batch carries `runs`, drawing-only `middle` runs and nullable `mainCurve`.
`mainCurve` is measured on the fit as given and is identical in every slice.
Only the slice starting at version 0 carries one `MiddleRun` per placement;
other slices carry an empty `middle` list. Each middle run has `layoutId`,
`before` (the fit's own T1 to L5 poses), `rodsIn` and `rods`. It uses the fit
without fit noise, with every drawn unknown it uses at the middle of its
range (quantile one half). Pedicle draws are unused and omitted. If any step
of the drawing run fails or its rods-in solve does not settle, `rodsIn` and
`rods` are both null; the runs over the versions are still returned. This adds
one correction run per placement to the first slice only.

```rust,ignore
let batch = load_bench::run_load_bench(&shapes::load_bench::BenchInput {
    fit,
    layouts: layouts.clone(),
    child,
    seed: 7,
    versions: 32,
    version_from: 0,
    version_count: 4,
})?;
let result = load_bench::compare_layouts(&shapes::load_bench::BenchCompareInput {
    layouts,
    batches: vec![batch], // supply every slice before showing a complete comparison
})?;
```

`compare_layouts` aggregates supplied runs. The first element of `layouts` is
the first placement. Layout identity and global version index pair the runs;
input order is not the pairing rule. Keep every placement on exactly the same
versions. Changing the ensemble size changes its stratification, so four draws
from a 32-version ensemble differ from a separate four-version ensemble.

The shared sampler is `spine_readings::readings::NamedSampler`, also re-exported
through `implant_layout::readings`. Geometry uses the same named dimensions as
placements. Bench-only dimensions have their own names. A global index retains
its draws in any batch split. No files, clocks, threads, environment or network
are used at run time; evidence is compiled into the crate, and transcendental
maths uses `libm`.

A refusal gives an existing engine error code with a short reason. Invalid
values are `case-invalid-value`; an empty fit is `pass-no-spine-found`; a
well-formed but unavailable computation is `tool-refused`. A solver cap is not
a settled physical answer. Read `settled` and `versionsUnsettled` alongside the
ranges. Unsettled runs keep only their computed before stop; no uncomputed
loads enter summaries. Their unused springback share is zero and must only be
read when `settled` is true. Incomplete estimate ensembles remain assumed,
and incomplete part ensembles are not called steady.

The result copies nullable `mainCurve` (`upper`, `apex`, `lower`) and the
middle drawing data from the supplied batches. At `before`, `middle` is the
fit's standing poses. At `rods-in`, `middle` is the seated poses and `rods`
holds the seated centre lines (`side`, `pointsMm`), top to bottom in patient
millimetres. These fields are null at later stops; seated poses and rods are
also null if the middle run does not settle. Without a placement's middle
run its drawing fields stay null, and without the first slice the result's
`mainCurve` stays null. A fit with no main curve also gives null. Comparison
refuses batches with different `mainCurve` values. The middle run is never
one of the versions: no estimate, part, count or ghost reads it. Per-part `demand` is an
anchor's force over bone-adjusted strength over settled versions, absent for
empty pedicles and other parts. At `before`, only shape estimates remain;
there are no load estimates, load paired counts or part shares. Load scales
start at the lowest loaded p10, without a zero from `before`.

`Estimate.sources` and `PartResult.sources` retain keys from the selected
mechanical, strength and fatigue evidence rows. Film-only values have no
sources. `StopResult.cyclesSources` retains the year-length assumption.
A part's sources cover its load and strength, including the loads behind its
shared denominator across stops and settled placements. Missing prior-row
provenance is detailed under the limits below.

## What is computed

The chain has one rigid body per vertebra and six springs at each disc. The
ground is at fitted L5's bottom centre with an identity frame because the fit
carries no sacrum pose; this is an explicit boundary convention. Its
rest shape is the standing fit. Joint translations are measured at the material
point midway between the facing endplate centres; rotations use the exact
principal rotation log relative to rest, below its half-turn cut.

Correction has three states:

1. **Seated and held, H.** Heads swivel and slide on rigid rod curves; the lowest
   station of each rod is pinned. The holder keeps zero twist about that rod's
   originally supplied chord, relative to its supplied placement. Its other
   five rigid freedoms remain free. Tissue energy chooses each vertebra's
   remaining turn about its two-head line. A project-selected 20 mm straight
   continuation beyond either contour end lets heads seat; a head reaching the
   continuation end is refused.
2. **Locked and let go, R, order 0.** Screws lock all six relative freedoms at
   their H values; hooks keep only three translation ties. Holders are removed.
   Rods are free rigid bodies. For screws only, the instrumented bodies move as
   one rigid block and outside joints return to rest. Balanced head wrenches
   are selected by the least rod strain energy, the elastic stiff-rod limit.
3. **Springback, order 1.** One prestressed linear elastic step about R lets
   the rods flex. The heads stay locked and there is no external holding torque.
   `stiffRodShare` is the norm of this step divided by the rigid correction,
   with rotations converted using the solver's 20 mm coordinate scale. It is
   not a percentage loss of Cobb correction.

The proposal's target is made by turning joints about their material disc
points, keeping the lowest instrumented pose. It introduces no target joint
translation. Thoracic kyphosis adjustment is spread only through the named
T5/T6–T11/T12 joints. Both rods in the first placement have zero extra overbend;
this version cannot represent permanent flattening of an overbent rod.

| Stop | Computation | Pose output |
|---|---|---|
| `before` | standing reference, without hardware load | standing poses |
| `rods-in` | R plus one elastic springback step, no gravity | corrected poses |
| `standing` | locked-in wrench plus passive standing increment | null; displayed shape stays at rods-in |
| `bending-forward` | locked-in wrench plus scaled standing increment | null |
| `year-of-walking` | standing/walking stress swing, Goodman conversion and annual steps | null |

These stop definitions and the prescribed load sites are project choices.
Daily loads are first-order solves about the seated state with its full
tangent, including prestress once. Walking and bending scale the same standing
response. They do not make a distinct flexion shape. Rod statics retains head
couples and moving-point terms. The year comparison uses a scalar surface
stress envelope, rather than following one material fibre. For section statics,
the elastic contour is linearly interpolated between transported head motions.
Drawing lines retain every contour knot and any occupied end continuation.
Their length can differ from the proposal's head-to-head length: head
translations are interpolated linearly between stations (first order), and
heads can seat on occupied end continuations.
Daily statics keeps first-order head and cut movement terms dP cross F0,
with the cut tangent transported by that interpolated head-motion gradient.
This stress calculation does not add a daily shape stop.

## Units, frames, signs and ordering

Patient coordinates are millimetres: x anterior, y to the patient's left, z
cranial, with x cross y = z. The origin is the centre of L5's lower endplate.
The fit and solver use radians; outward angle estimates use degrees. Forces
are N, moments N·mm, stresses MPa = N/mm². Springs use N/mm and N·mm/radian;
evidence rotational entries in N·mm/degree are multiplied by 180/pi.

Positive x rotation tips the column toward the patient's right; positive y
rotation tips it anterior; positive z turns anterior toward the patient's left.
Head wrenches inside the solver act **on the rod**. The force on the vertebra
has the opposite sign. Positive pull-out is force on the vertebra projected
opposite the screw's inward axis. The outward anchor result is its resultant
force magnitude, including locked-in and daily components before taking norms.

Runs are placement by placement, then global version order. Stops follow
`BenchStop`. Anchor arrays follow the placement's anchor order, with zero for
empty pedicles. Rod spans are left top to bottom, then right top to bottom.
Solver rod/head arrays use `RigidState::anchor_order`; they must be mapped back
to proposal order. Pose arrays run top to bottom.

Main Cobb and apical twist use the same selected curve ends and apex across
versions and across all compared placements, including alternatives with
different end levels. Any part outside a construct only follows the model's
standing-rest joints; spontaneous unfused correction cannot be read from it.
Kyphosis is T5 upper
endplate sagittal tilt minus T12 lower endplate tilt, positive for kyphosis.
For front-film-only fits, sagittal geometry is a typical spine, and inferred
twist is unmeasured. Those typical values must never be presented as measured
readings. `StopValues` has mandatory scalar angles without nullable status
fields, so one-film runs carry finite typical scenario angles; comparison
conservatively tags every kyphosis and apical-twist estimate assumed, even
when two films were used. When `fit.lengths` is `typical-size`, mechanical lever arms belong to
the drawn typical-sized child, not calibrated distances on this child.

## Reading ranges, shares and counts

Bands are p10, p50 and p90, using linear interpolation at p × (N−1). A band
holds the middle eight in ten of the engine's versions, not eight in ten
patients. A part's share is divided, in each version, by the largest load of
its kind across every stop and every compared placement. This common scale
keeps placement pictures comparable. For rod spans the wire supplies use
(stress/yield), so shares compare use; whole-rod shares compare fatigue ratios.
If a rod's fatigue ratio is missing, no finite peak or global whole-rod share
is inferred from the remaining known ratios. Zero loads do not create a force.

Tags are computed per estimate using project thresholds:

- `set-by-the-films`: p90−p10 is at most 2 degrees for an angle or 10% of p50
  for a ratio.
- `comparison-only`: the band is wider, but its paired difference against the
  first placement keeps its sign in at least 29 of 32 versions (90%).
- `assumed`: otherwise, and always for absolute forces and counts past
  published strength. The batch contract does not carry measurement-tier
  provenance: kyphosis and apical twist are therefore always tagged assumed,
  including sagittal versions outside the stiff-rod range. A narrow prior does
  not turn absolute force into a measured result.

`mostlyFrom` is the sampled unknown with largest absolute Spearman rank
correlation with that estimate. Ties receive equal average ranks. Dot order
sorts by the unknown most correlated with the **paired difference**. These
associations describe the chosen ensemble, not causal attribution. Geometry
and rib cage use the strongest individual component. `mostlyFrom` and
`sortedBy` are null when no unknown has nonzero rank correlation with the
result; dots then follow global version order. Ghosts are
actual version poses nearest the p10 and p90 main-Cobb ranks; a pose is never
interpolated between spines.

A paired count compares version k of a placement with version k of the first
placement. Differences within 2 degrees for curve angles or 10% of the first
placement's load are ties. The app may call a count clear only at 24/32 or
more; the engine carries counts, and the app supplies words. The maximum
binomial standard error at 32 versions is sqrt(0.25/32), about 0.088. These
margins, count cutoffs and tag cutoffs are project thresholds, not clinical
thresholds. A count past published strength is also an assumption; it is not a
failure probability.

`PairedCount.outcomes` contains one `lower`, `higher` or `tie` for each dot,
following `order`. Its outcome totals equal the corresponding counts.
`clearAt` is the clear-count
threshold of three quarters of the number of entries in `order`, rounded
up. The existing `lower`, `higher`, `ties` and dot order are unchanged.

## Embedded tables and sources

All numerical evidence is in strict typed JSON with source keys, units,
published or own-inference basis, and explicit absences. `Evidence::read()`
opens no run-time files. Each source key resolves through
[`evidence/sources.json`](evidence/sources.json) and the full
[bibliography and table qualifications](evidence/README.md). Source access
below is the supplied reading status; bibliographic confirmation does not
establish a number or make adult or animal findings child measurements.
Comparison rows remain alongside selected rows rather than silently replacing
them.

| Table | Inputs used or retained | Source and qualification |
|---|---|---|
| `joint-springs.json` | T1/2–L4/5 translation AP/ML/CC 1392/294/341 N/mm; rotation about AP/ML/CC 448/644/738 N·mm/degree. L5/S1 700/190/190 and 222/410/410. | Tachi 2021, n=47; borrowed adult lumbar stiffness, uniformly applied in that planner [peer-reviewed, full text]. |
| same | Release: axial rotation −17%, flexion −15%, coronal −3.8%, compression −14%. | Wang 2017; cited cadaver calibration targets, not measurements from its ten patients [peer-reviewed, full text]. |
| same | Preload factor 1.71–2.11. Overall stiffness ×0.65–5.6 (assumed solver calibration). Thoracic lateral and axial rib factors. Adult disc and thoracic comparisons. | Gardner-Morse and Stokes 2004 [peer-reviewed, abstract]; Petit 2004 [peer-reviewed, abstract]; Watkins 2005 [peer-reviewed, abstract], Mannen 2018, Liebsch 2020, Stokes and Gardner-Morse 2016 and Wilke 2017 [peer-reviewed, full text]. The scale is calibrated against Salmingo 2013 head forces; rib ranges are own priors. Flexion bounds are derived explicitly from Mannen's with/without-rib values 2.29/2.12 and 2.97/1.39; the literal rounded range is not a published table cell. |
| `rod-materials.json` | Titanium E=114 GPa, yield 896–1034 and ultimate 965–1103 MPa; CoCr E=240, yield 500–1500, ultimate 900–1540; steel E=193, yield 170–310, ultimate 540–1000. | Prasad 2017 [peer-reviewed, full text]. Titanium strengths use body text because its Table 1 pair contradicts itself. One quantile samples yield and ultimate together, keeping yield below ultimate. Steel condition is not stated. |
| same | Alternative planner/coupon material sets, textbook ranges and computed section comparisons. | Tachi 2021; Little 2013; Jeong 2025; Song 2021; Ohrt-Nissen 2018; Abe 2015 and other source keys retained in that table [peer-reviewed; access in bibliography]. They are distinct sets, not mixed material properties. Section arithmetic is own inference. |
| `anchor-strength.json` | T1–T8 screw 481–611 N; T9–T12 screw 807 N; hook 321 N; lumbar screw 1056.4 N. | Lehman 2003, 30 paired adult thoracic vertebrae; Liljenqvist 2001, 9 adult spines/90 tests; Soshi 1991, lumbar cadavers [peer-reviewed, abstracts]. T1–T8 assignment and borrowing the hook region are own choices. |
| same | Bone factor 1 in two thirds; otherwise 0.47–1. Animal, trajectory, depth, density and augmentation comparisons retained. | Bone mixture is own inference; Soshi's grade-I ratio 495.6/1056.4 gives its broad floor. Cheng 2000 and Yip 2016 describe low bone density in AIS [peer-reviewed; access in bibliography]. Animal findings do not establish child pull-out strength. |
| `daily-loads.json` | Head/neck 0.081 BW, each arm/hand 0.056 BW. Trunk fractions 0.010 at T1 to 0.026 at L5, interpolated. Centroid anterior offset 28±5.6 mm. | Winter attributed through Keenan 2015 [peer-reviewed, full text; textbook unopened]; Pearsall 1996 and Keenan 2014 [peer-reviewed, abstracts]; Gangnet 2003, n=34 [peer-reviewed, abstract]. Interpolation, load sites and reusing an unsigned 3D gravity-line distance as a sagittal offset are own choices. |
| same | Walking 1.28–1.71×standing; forward bending 2.2×; weight 29–63 kg if absent. Other adult telemetry activities retained. | Rohlmann 2002, n=10 internal-fixator patients and Rohlmann 2014 gait, n=5 vertebral-body-replacement patients [peer-reviewed, abstracts]; Wilke 1999, one adult [peer-reviewed, abstract]; Schmid 2020 [peer-reviewed, full text]. Joining unlike telemetry quantities is an own prior. |
| same | Adult activity edges 5000/7500/10000/12500 steps/day; finite absent-activity prior 8000–16000; 365 days/year. | Tudor-Locke 2008 [peer-reviewed, abstract] and 2011 [peer-reviewed, full text]. Youth cutoffs are not established. Adult very-low and very-active bands have an open boundary; no finite missing endpoint is invented. The bench refuses either open-ended activity category until a finite prior is supplied. Annual arithmetic and absent-activity prior are own choices. |
| `fatigue.json` | Ti/CoCr construct run-outs 250/290 N, 5.5 mm rods, 5 million cycles. Historical Goodman-equivalent references 169/191 MPa; contour factor 0.6–0.8. Coupon comparisons retained. | Vieweg 2023 [peer-reviewed, full text]; Slivka 2013 [peer-reviewed, abstract]. The 40 mm fixture arm/equal sharing conversion and extrapolation of contour loss to CoCr and other diameters are own inference. Runtime ultimate strengths are sampled from Prasad, not the historical calibration's 900/1200 MPa. Steel has no construct-fatigue reference. |
| `sources.json` | Publication identifiers, sample and population qualifications, source access and known omissions. | Bibliographic metadata for the papers above and the retained comparison rows. The project-assumptions key is own inference, not a paper. |

The assumed 0.65–5.6 stiffness prior calibrates this solver with 5.5 mm
cobalt-chrome rods and no overbend against published peak head forces of
192–439 N (Salmingo 2013, n=6, PMID 23273729; author manuscript read in full).
It is not a measurement of the child.

A uniform tissue scale leaves order-0 geometry unchanged and scales its
forces; it also affects springback and daily load sharing. Rod
beam Poisson ratio 0.3 and shear coefficient 0.9 are explicit project choices,
not Prasad material claims. Screw capacity is not scaled by catalogue size
because the requisite reference screw dimensions are missing.

### Paper identifiers

The following records come from the embedded source ledger. Full titles,
coauthors, journal locations, populations and retained comparison citations
are in `evidence/sources.json` and `evidence/README.md`.

- Tachi H et al. (2021), `tachi-2021`: PMID 34712654; [DOI 10.3389/fbioe.2021.746902](https://doi.org/10.3389/fbioe.2021.746902) [peer-reviewed, full-text].
- Wang X et al. (2017), `wang-2017`: PMID 28428982; [DOI 10.1186/s13013-017-0120-4](https://doi.org/10.1186/s13013-017-0120-4) [peer-reviewed, full-text].
- Gardner-Morse MG et al. (2004), `gardner-morse-2004`: PMID 14706323; [DOI 10.1016/j.jbiomech.2003.10.003](https://doi.org/10.1016/j.jbiomech.2003.10.003) [peer-reviewed, abstract].
- Stokes IAF et al. (2016), `stokes-gardner-morse-2016`: PMID 26900035; [DOI 10.1016/j.jbiomech.2016.01.035](https://doi.org/10.1016/j.jbiomech.2016.01.035) [peer-reviewed, full-text].
- Wilke HJ et al. (2017), `wilke-2017`: PMID 28520819; [DOI 10.1371/journal.pone.0177823](https://doi.org/10.1371/journal.pone.0177823) [peer-reviewed, full-text].
- Watkins R et al. (2005), `watkins-2005`: PMID 15928553; [DOI 10.1097/01.brs.0000164257.69354.bb](https://doi.org/10.1097/01.brs.0000164257.69354.bb) [peer-reviewed, abstract].
- Mannen EM et al. (2018), `mannen-2018`: PMID 29852313; [DOI 10.1016/j.jmbbm.2018.05.019](https://doi.org/10.1016/j.jmbbm.2018.05.019) [peer-reviewed, full-text].
- Liebsch C et al. (2020), `liebsch-2020`: PMID 32117927; [DOI 10.3389/fbioe.2020.00046](https://doi.org/10.3389/fbioe.2020.00046) [peer-reviewed, full-text].
- Petit Y et al. (2004), `petit-2004`: PMID 14977223; [DOI 10.1007/bf02351011](https://doi.org/10.1007/bf02351011) [peer-reviewed, abstract].
- Prasad K et al. (2017), `prasad-2017`: PMID 28773240; [DOI 10.3390/ma10080884](https://doi.org/10.3390/ma10080884) [peer-reviewed, full-text].
- Little JP et al. (2013), `little-2013`: PMID 23680391; [DOI 10.1186/1748-7161-8-9](https://doi.org/10.1186/1748-7161-8-9) [peer-reviewed, full-text].
- Ohrt-Nissen S et al. (2018), `ohrt-nissen-2018`: PMID 29991241; [DOI 10.14245/ns.1836050.025](https://doi.org/10.14245/ns.1836050.025) [peer-reviewed, full-text].
- Jeong BC et al. (2025), `jeong-2025`: PMID 41383715; [DOI 10.3389/fbioe.2025.1673061](https://doi.org/10.3389/fbioe.2025.1673061) [peer-reviewed, full-text].
- Song M et al. (2021), `song-2021`: PMID 34155224; [DOI 10.1038/s41598-021-90686-6](https://doi.org/10.1038/s41598-021-90686-6) [peer-reviewed, full-text].
- Lehman RA et al. (2003), `lehman-2003`: PMID 14501914; [DOI 10.1097/01.brs.0000087743.57439.4f](https://doi.org/10.1097/01.brs.0000087743.57439.4f) [peer-reviewed, abstract].
- Liljenqvist U et al. (2001), `liljenqvist-2001`: PMID 11383294 [peer-reviewed, abstract].
- Soshi S et al. (1991), `soshi-1991`: PMID 1750009; [DOI 10.1097/00007632-199111000-00015](https://doi.org/10.1097/00007632-199111000-00015) [peer-reviewed, abstract].
- Cheng JC et al. (2000), `cheng-2000`: PMID 10934658; [DOI 10.1359/jbmr.2000.15.8.1587](https://doi.org/10.1359/jbmr.2000.15.8.1587) [peer-reviewed, abstract].
- Yip BHK et al. (2016), `yip-2016`: PMID 27991528; [DOI 10.1038/srep39220](https://doi.org/10.1038/srep39220) [peer-reviewed, full-text].
- Keenan BE et al. (2015), `winter-via-keenan-2015`: PMID 26681978; [DOI 10.1186/s13013-015-0060-9](https://doi.org/10.1186/s13013-015-0060-9) [peer-reviewed, full-text].
- Pearsall DJ et al. (1996), `pearsall-1996`: PMID 8678352; [DOI 10.1007/bf02667349](https://doi.org/10.1007/bf02667349) [peer-reviewed, abstract].
- Keenan BE et al. (2014), `keenan-2014`: PMID 24973826; [DOI 10.1016/j.clinbiomech.2014.06.002](https://doi.org/10.1016/j.clinbiomech.2014.06.002) [peer-reviewed, abstract].
- Gangnet N et al. (2003), `gangnet-2003`: PMID 13680185; [DOI 10.1007/s00276-003-0154-6](https://doi.org/10.1007/s00276-003-0154-6) [peer-reviewed, abstract].
- Rohlmann A et al. (2002), `rohlmann-2002`: PMID 11784277; [DOI 10.1093/ptj/82.1.44](https://doi.org/10.1093/ptj/82.1.44) [peer-reviewed, abstract].
- Rohlmann A et al. (2014), `rohlmann-2014-gait`: PMID 24211089; [DOI 10.1016/j.gaitpost.2013.10.010](https://doi.org/10.1016/j.gaitpost.2013.10.010) [peer-reviewed, abstract].
- Wilke HJ et al. (1999), `wilke-1999`: PMID 10222525; [DOI 10.1097/00007632-199904150-00005](https://doi.org/10.1097/00007632-199904150-00005) [peer-reviewed, abstract].
- Schmid S et al. (2020), `schmid-2020-ais`: PMID 32195239; [DOI 10.3389/fbioe.2020.00159](https://doi.org/10.3389/fbioe.2020.00159) [peer-reviewed, full-text].
- Tudor-Locke C et al. (2008), `tudor-locke-2008`: PMID 18562971; [DOI 10.1249/mss.0b013e31817c7133](https://doi.org/10.1249/mss.0b013e31817c7133) [peer-reviewed, abstract].
- Tudor-Locke C et al. (2011), `tudor-locke-2011`: PMID 21798014; [DOI 10.1186/1479-5868-8-78](https://doi.org/10.1186/1479-5868-8-78) [peer-reviewed, full-text].
- Vieweg U et al. (2023), `vieweg-2023`: PMID 36685708; [DOI 10.1016/j.bas.2022.101708](https://doi.org/10.1016/j.bas.2022.101708) [peer-reviewed, full-text].
- Slivka MA et al. (2013), `slivka-2013`: PMID 27927364; [DOI 10.1016/j.jspd.2013.08.004](https://doi.org/10.1016/j.jspd.2013.08.004) [peer-reviewed, abstract].
- Abe Y et al. (2015), `abe-2015`: PMID 25810754; [DOI 10.1186/1748-7161-10-s2-s2](https://doi.org/10.1186/1748-7161-10-s2-s2) [peer-reviewed, full-text].

## Limits that travel with every result

1. This is a planning preview for education and research, not a medical device
   and not clinical advice. The app's notice remains in place word for word.
2. Screw sizes come from published tables for the child's age, level and side
   of the curve; this spine's pedicles are not measured. One standard deviation
   of pedicle width, 1.0–1.6 mm, exceeds the 0.5 mm screw grid step. A CT would show
   the real pedicle. Population tables and their sources belong to
   [`implant-layout`](../implant-layout/README.md).
3. The displayed bone shapes are one adult's vertebrae shaped to the fit.
   Entry points follow tables, not landmarks measured on this bone.
4. Films cannot identify this spine's stiffness. Forces scale with it, so
   absolute forces are assumed. Compare placements with that uncertainty in
   view, rather than treating an absolute force as a measured outcome.
5. Rods flex but never bend permanently. Correction and forces are upper-end
   elastic estimates; published work reports that omitting permanent rod
   deformation overstates both. Versions outside the stiff-rod range are
   counted. This model has not established an individual correction.
6. Joint stiffness comes from adult cadavers and one published planner's
   table; no child's tissue stiffness is measured here. Strengths come from
   adult bone and young animals.
7. Daily loads omit muscles and use adult telemetry multiples. Every activity
   is the same passive standing load made bigger.
8. A year of walking compares rod swings with levels rods survived on a test
   bench. It gives no lifetime and no date.
9. Fit noise is drawn independently for each vertebra because the fit exports
   no joint covariance. This overstates neighbouring-joint noise.
10. Level names may be one off. The placement panel carries the changes under
    alternative numbering; a bench comparison assumes the supplied names.
11. A placement is not saved with the case.
12. Published simulators report about 2–5 degrees of Cobb error and do not
    establish force estimates in patients. This model has not been compared
    with any patient.

Further limits apply. Outside the rods, joints keep their standing rest shape;
spontaneous correction of an unfused curve and body rebalancing are absent.
Shape changes outside instrumented levels carry no spontaneous-correction
interpretation. Tightened heads never slip; hooks
are two-way translation ties without one-way contact. Growth, fusion timing,
acute screw–bone compliance and locked-in force relaxation are absent. The
fixed-work nonlinear solver finds a regular local constrained minimum; it
does not establish global uniqueness. A single elastic step is not full
nonlinear rod equilibrium.

With one front film, forward lean, depth position, side wedge and inferred
twist come from the built-in typical spine. They remain assumptions, and a
computed load does not make them this child's measurements. With no film
scale, the lever arms are those of a typical-sized child. The same assumptions
must be kept for every compared placement. Density and contour changes can
change which local constrained state is reached; a published-direction probe
is plausibility, not truth about this child's spine.

Batches do not retain the child's activity or whether weight was supplied.
Comparison therefore omits the selected `steps-low`, `steps-typical` or
`steps-active` row (`tudor-locke-2008`), or the absent-activity
`steps-if-absent` row and its `child-steps` basis (`tudor-locke-2011`), from
`cyclesSources`. The `project-assumptions` key is still present for the
year-length row; it does not establish which step prior was used. Comparison
also omits the `weight-if-absent` row (`schmid-2020-ais`) from daily-load
estimates and parts. It cannot infer these prior choices from sampled values.
Step counts do not enter the rod swing amplitude ratio.

The T1–T8 screw strength quantile is drawn, but it is not among the named
unknowns available to `mostlyFrom` or `sortedBy`. Those fields can describe
associations only with the unknowns carried into comparison; a null does not
rule out movement with that strength draw.

## Numerical checks

Linear correction and daily solves recover rod strains in the moving frame
before applying stiffness. Up to eight improving refinement steps check the
same body and free-frame equations at the unchanged 1e-9 relative tolerance;
a non-positive tangent or residual above tolerance remains unsettled.

The suite includes closed-form spring/beam/statics/Goodman checks, balance and
mirror checks, stiffness and weight scale laws, no-force fitting rods,
three-state correction checks, batch identity, paired version identity, tags,
shares and serialised contract checks.

`bands_cover_the_true_geometry` fits noisy made-up front and side marks, keeps
one supplied three-body rod contour fixed, and compares a true-geometry
order-0 signed apical force with p10–p90 from sampled fitted geometries. The
small CI check uses two cases with eight versions each and requires at least
one covered result: two misses have probability 0.2²=0.04 at nominal 80%
coverage. The ignored full experiment uses 64 cases and 32 versions. Its
tolerance is three binomial standard errors sqrt(0.8×0.2/64), plus one trial's
resolution. The recorded full run covers 52/64 cases (81.25%) against 80%,
within its 0.165625 tolerance.
The small check is a smoke check; neither experiment establishes coverage in
patients or covers uncertainty in the mechanical model itself.

`published_directions_hold` records published-direction plausibility:
concave versus convex apical pull-out, lower density, unequal rod contours
and the circular-section diameter-to-fourth-power law. The static prescribed
contours do not reproduce a surgeon's transient simultaneous rod rotation.
In particular, turning a straight rod about its long axis is invisible to
this fixture. A source-specific observed direction is not a universal law
for an arbitrary static boundary condition.

The tests compute and report whether each direction agrees. They assert
independent physical conditions: every body's force and moment balance,
each rod's resultant plus holder torque, the ground reaction, finite results,
rotation orthogonality and the signed projection bound. The insertion axis
turns with its seated vertebra. The sparse case removes alternating bilateral
levels while retaining both ends and the apex pair: six of ten heads, rather
than a claim of exactly half the screws. These repairs do not establish
reproduction of the published manoeuvres. No prior, material or contour
amplitude was tuned to obtain a direction.

The current static five-body fixture gives dense concave/convex signed
apical pull-out of 502.692/−2459.303 N. The bilateral alternate-level case gives
−473.294 N at the concave apex. Thus the concave comparison agrees and the
density comparison disagrees with the requested published directions.
Increasing the concave posterior contour bow from 0.5 to 1.0 mm changes the
largest H force from 47654.712 to 3537.856 N; that direction also disagrees.
Both comparisons use the same initialisation policy. An explicit check of the
old inputs with and without that initial guess gave the same solutions to
round-off; initialisation does not explain away those observations.
These numbers are computed toy-case results, not published values. Passing
the equilibrium checks does not mean the published force trends were
reproduced. The diameter law passes: (6/5.5)^4 is about 1.416, a 42% increase
of bending EI.

The requested contour-direction source has no Wang 2016 record in the
embedded ledger, so this README does not supply a reconstructed citation.
The requested Pillot 2026 permanent-deformation comparison likewise has no
record there; the elastic upper-end limitation is retained without inventing
an identifier. Earlier values computed with a fixed patient-frame insertion
axis and unilateral apex removal did not measure the intended comparison.
