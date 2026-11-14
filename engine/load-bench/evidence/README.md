# Load bench evidence

These tables transcribe values from the publications listed in `sources.json`.
For a published value, each cell's `section` gives the publication locator,
or says `not recorded` where none is known. For a project calculation or
choice, it describes the basis. Each source's `read` records how much of it
was read.
The tables are inputs for an education and research preview; the bundled
data are not a checked evidence ledger.

| File | Holds |
|---|---|
| `joint-springs.json` | Tachi joint tables and defined moment axes; cited release calibration; preload and cadaver comparisons; rib EZS with sample qualifications; corrected Wilke ROM and own secants |
| `rod-materials.json` | Every original material set, contradictory Prasad Table 1 values, body-text strengths, selected strength pair, separate Poisson values and copied computed section table |
| `anchor-strength.json` | Exact published pull-out forces; own osteoporosis ratios with their inputs; regional screw/hook references; distinct animal/foam comparisons and uncertainties |
| `daily-loads.json` | Borrowed masses, raw gravity-line measurements beside the sagittal prior, distinct telemetry quantities, literal activity edges and rounded annual-cycle comparisons |
| `fatigue.json` | Published construct inputs, historical own Goodman calibration, all referenced coupon conditions, contour factor and known absences |
| `sources.json` | Bibliographic records for the papers, read status, sample/population qualifications, identifiers, reading qualifications and comparison citations |

All numerical objects are typed and refuse unknown fields. Each cell carries its
term, value, unit, source, publication locator, basis and approximation flag. Mean, SD,
SE and inequality endpoints are separate terms. `bench` selects an input or prior;
`beside` preserves comparisons; `input` retains an inference's inputs. No lookup
interpolates, converts units, supplies missing numbers or picks a nearby row.

`Evidence::read()` reads six compiled `include_str!` strings. `table`, `row`,
`value`, `source` and `sources` expose the ordered data; exact missing row/term
lookups return `None`. `nmm_per_degree_to_nmm_per_radian(x)` is the one explicit
unit conversion, `x * 180 / pi`, tested against a closed form. It does not remap
axes. Published units, percentages, cycles, reversals and millions remain as read.

## Selected rod strength pairs

`Evidence::bench_strengths(RodMaterial)` returns a `StrengthPair` of four sourced
MPa endpoints: `yield_lower`, `yield_upper`, `ultimate_lower`, `ultimate_upper`.
It selects the following rows, without sampling or interpolation:

| Material | Row | Yield MPa | Ultimate MPa |
|---|---|---|---|
| Titanium alloy | `titanium-alloy-body-text` | 896–1,034 | 965–1,103 |
| Cobalt chrome | `cobalt-chrome` | 500–1,500 | 900–1,540 |
| Stainless steel | `stainless-steel` | 170–310 | 540–1,000 |

Each version uses one common quantile for both endpoints' ranges. This
preserves yield below ultimate: endpoint margins are 69/69 MPa for Ti, 400/40
for CoCr, and 370/690 for steel. Independent draws are not the chosen prior.
Sampling belongs to the version implementation.

The old `titanium-alloy` Table 1 row remains `beside`: its printed yield upper
1,103 MPa exceeds its tensile upper 965 MPa. It is not the bench strength pair.
Its E = 114 GPa remains the selected titanium modulus. The body-text E = 115 GPa
is retained separately. Body-text CoCr yield 448–1,606 and tensile 655–1,896 MPa
are comparisons; CoCr uses the Table 1 pair above. Prasad labels steel simply
“Stainless steel 316L”, without a material-condition qualifier.

The separate Tachi set has E = 420 GPa and Poisson ratio 0.3; Little has E = 108
GPa, yield 390 MPa and Poisson ratio 0.3; Jeong has E = 108 GPa, yield 864 MPa
and ratio 0.342; Song has E = 113 GPa and ratio 0.30. None supplies a Poisson
ratio for a Prasad set. No shear modulus is derived or supplied.

## Corrections and provenance

- Tachi's moments 1/2/3 are about AP/x (lateral bending), ML/y (flexion-extension)
  and CC/z (axial rotation), as its Table 1 headings state. Joint values are
  borrowed by Tachi from Argoubi and Shirazi-Adl 1996,
  an adult lumbar source, then used uniformly by the planner.
- The Stokes Table 3 footnote defines x as posterior-anterior shear or lateral
  bending, y as lateral shear or flexion-extension, z as compression-tension or
  axial rotation. The 642 N column is a different series: eight segments from
  seven spines, five male/two female, mean age 41, at 0.01 Hz; mean preload
  642 ± 132 N. The other columns belong to the four-female-donor preload series.
- Wang's four signed release percentages are calibration targets cited from
  cadaver studies, not Wang's own measurements. They are distinct from Tachi's
  after-facetectomy table. Remaining fractions are computed by the bench.
- Wilke's 5.2–5.7 degree AR range covers T2-T3 through T5-T6. Exact term keys
  are now `t2-t6-ar-lower/upper` in `thoracic-rom` and `thoracic-secants`.
  T6-T7 remains separate at 5.9 degrees; the old `t2-t7-ar-*` keys are absent.
  The affected secants are recomputed as 7.5/5.7 and 7.5/5.2 Nm/degree, stored
  without decimal rounding and marked own inference beside the published inputs.
  Other recorded secants are unchanged. Lateral bending still covers T2–T7.
- Mannen's intact means use eight specimens; no-ribs EZS uses seven after one
  exclusion. Units are not printed in the paper and remain `unstated`.
- Soshi's normal/grade-I/grade-II forces are 1,056.4/495.6/269.5 N at 7.0 mm.
  The abstract says cadaveric lumbar vertebrae and does not state adulthood.
  Ratios 0.47/0.26 are own arithmetic, rounded from 495.6/1056.4 and 269.5/1056.4,
  with published inputs in the same row. The 0.47 bone-prior floor remains a
  deliberately broad project choice, not a measured paediatric strength law.
- Liljenqvist's 600 N comparison is a supralaminar hook at T9–T12. The 321 N
  pedicle-hook setting is T4–T8. Reference thoracic screw dimensions and Soshi
  reference length remain absent; no capacity scaling by screw size is added.
- Yilmaz's pigs were six weeks old at surgery, with pull-out measured three
  months later. Two animals supplied data; three were operated and one lost.
  Rounded dilated mean 379 N remains beside the abstract's 378.7 N.
  Le Cann's rounded 420/1,341 N likewise remain beside full-text group-mean
  extremes 419.9/1,341.2 N. No cross-population animal range is fitted.
- Gangnet's raw unsigned 3D body-centre/gravity-line distance is 28 mm (SD 5.6).
  The left lateral distance is 7.6 mm (SD 1.6), in 34 healthy upright standing
  subjects taken here as adults; the abstract gives no ages. Both
  raw measures are in `slice-offset`. Its generic `mean/sd` 28/5.6 cells now
  explicitly carry own-inference provenance for the sagittal anterior reuse.
  Frontal-at-body-centre is also a project choice and ignores the lateral mean.
- The walking lower 1.28 concerns internal-fixator load; upper 1.71 concerns
  resultant force on a vertebral body replacement, each normalised to its own
  standing reference. Sampling between them is a cross-quantity project prior.
- Mass fractions 0.081/0.056 are confirmed as printed in Keenan 2015, which is
  cited in full. It attributes them to Winter DA, *Biomechanics and Motor
  Control of Human Movement*, 4th ed, John Wiley & Sons, 2009. Winter's table
  itself was not opened. No memory-only alternative fraction is used.

## Retained assumptions and absences

Films do not establish absolute joint stiffness. The assumed global 0.65–5.6
log-uniform range calibrates this solver with 5.5 mm cobalt-chrome rods and no
overbend against the published 192–439 N peak head-force range; it is not a
measurement of the child. Salmingo et al. (2013), Clin Biomech, n=6,
PMID 23273729, was read as the full author manuscript.
Its forces were worked back from deformation of 6 mm titanium-alloy rods.
The scale endpoints remain own inference; the published forces are the
calibration target. The older Petit factors use another baseline and remain
beside this prior. Rib axial/lateral multipliers remain
own priors; literal flexion endpoints 1.08/2.14 remain absent rather than newly
inferred. Adult/animal data do not calibrate individual paediatric tissue.

No intermediate T2–L4 mass cells, muscle share, youth activity cut points, finite
lower very-low or upper very-active step boundary, acute screw–bone compliance,
contoured-rod S–N slope, fusion timing, locked-in relaxation law or stainless
construct-fatigue level is filled. Each table lists its numerical absences.

Adult step bands keep 7,499/9,999/12,499 and inclusive 12,500; rounded project
step bands stay beside them. Annual counts remain in million cycles/year.
Coupon million reversals, individual reversals and the strict >10⁶-cycle run-out
definition keep their own units. Telemetry absolute forces are not standing
multiples; >1,200 N is a threshold exceeded, not an upper bound. Pull-out
multipliers are not established to stack. Sairyo's abstract gives 852.5 N and a
rounded comparison gives 853 N; both remain separate.

Construct calibration retains Vieweg's 250/290 N run-outs, 5.5 mm rods, R = 10,
5 Hz and 5 million cycles beside the unverified 40 mm arm/equal two-rod sharing.
The copied own 169/191 MPa equivalents use historical assumed UTS 900/1,200 MPa.
Those UTS cells reproduce that calibration only: they are not the runtime UTS
prior, which now comes from `bench_strengths` at the same quantile as yield.
No new life extrapolation is supplied. Slivka's 20–40% Ti/steel contour loss and
the derived 0.6–0.8 factor do not establish the CoCr/5.5 mm extrapolation.

The computed rod section table remains copied own inference with its reference
E/yield inputs; it is not silently recalculated. The UHSS comparison remains
200 ksi beside an approximate conversion of 1,380 MPa. The separate 2015 flexion
record's Dreischarf/Rohlmann naming disagreement remains documented.

## Bibliography and read status

`Citation` carries authors/year/title/journal plus volume/issue/pages. `Source`
adds PMCID and `recordNote`. `recordNote` says what the bibliographic
confirmation covered (`bibliographic record and read status` or `bibliographic
record only`); `read` records how much of the publication was read. A PMC link
alone never raises read status. The Keenan alias receives the same full record
as the Winter-through-Keenan key, while preserving the unopened textbook
qualification.

The main source set identifies 34 unique papers, filling 35 source entries with
the Keenan alias. Every identified paper has a title and supplied PMID/DOI.
The 22 additional source keys include the Keenan alias. All 56 publication source entries have
full author/title/journal/location records and a PMID. Only Liljenqvist has no
DOI in its supplied record. Extra-source notes state “Bibliographic record
confirmed; numbers not independently checked”; their recorded read status is
unchanged. Metadata confirmation does not turn into numerical verification.
No citation is reconstructed from memory.
The project-assumptions entry is not a publication and has no paper identifier.

- `project-assumptions` — Spine Workbench (2026). Placement and load bench assumptions. journal not recorded. No publication identifier supplied. [own-inference; secondary]; n = not given. No measured population; explicit project choices and calculations.

- `tachi-2021` — Tachi H, Kato K, Abe Y, Kokabu T, Yamada K, Iwasaki N, Sudo H (2021). Surgical Outcome Prediction Using a Four-Dimensional Planning Simulation System With Finite Element Analysis Incorporating Pre-bent Rods in Adolescent Idiopathic Scoliosis: Simulation for Spatiotemporal Anatomical Correction Technique. Front Bioeng Biotechnol 9:746902. PMID 34712654; DOI 10.3389/fbioe.2021.746902; PMCID PMC8546212. [peer-reviewed; full-text]; n = 47. AIS planner comparison against postoperative CT. ([record](https://europepmc.org/abstract/MED/34712654)) Record: bibliographic record and read status.

- `wang-2017` — Wang X, Larson AN, Crandall DG, Parent S, Labelle H, Ledonio CGT, Aubin CE (2017). Biomechanical effect of pedicle screw distribution in AIS instrumentation using a segmental translation technique: computer modeling and simulation. Scoliosis Spinal Disord 12:13. PMID 28428982; DOI 10.1186/s13013-017-0120-4; PMCID PMC5393020. [peer-reviewed; full-text]; n = 10 patients; 50 simulations. AIS multibody simulations; release terms borrowed from the simulator. ([record](https://europepmc.org/abstract/MED/28428982)) Record: bibliographic record and read status.

- `gardner-morse-2004` — Gardner-Morse MG, Stokes IA (2004). Structural behavior of human lumbar spinal motion segments. J Biomech 37(2):205-212. PMID 14706323; DOI 10.1016/j.jbiomech.2003.10.003. [peer-reviewed; abstract]; n = not given. Intact human lumbar segments with posterior elements; 250 and 500 N preload. Own abstract passage gives no n. ([record](https://europepmc.org/abstract/MED/14706323)) Record: bibliographic record and read status.

- `stokes-gardner-morse-2016` — Stokes IAF, Gardner-Morse M (2016). A database of lumbar spinal mechanical behavior for validation of spinal analytical models. J Biomech 49(5):780-785. PMID 26900035; DOI 10.1016/j.jbiomech.2016.01.035; PMCID PMC4801716. [peer-reviewed; full-text]; n = 8 per series. Lumbar discs without posterior elements. Preload series: L2-3/L4-5, four female donors aged17–58. Frequency series: eight segments from seven spines, five male and two female, mean age41;642N column measured at0.01Hz. ([record](https://europepmc.org/abstract/MED/26900035)) Record: bibliographic record and read status.

- `wilke-2017` — Wilke HJ, Herkommer A, Werner K, Liebsch C (2017). In vitro analysis of the segmental flexibility of the thoracic spine. PLoS One 12(5):e0177823. PMID 28520819; DOI 10.1371/journal.pone.0177823; PMCID PMC5433776. [peer-reviewed; full-text]; n = 68 functional units from29 donors;6–7 per level. Human cadaver thoracic functional units with costovertebral joints kept; ±7.5 Nm. ([record](https://europepmc.org/abstract/MED/28520819)) Record: bibliographic record and read status.

- `watkins-2005` — Watkins R, Watkins R, Williams L, Ahlbrand S, Garcia R, Karamanian A, Sharp L, Vo C, Hedman T (2005). Stability provided by the sternum and rib cage in the thoracic spine. Spine (Phila Pa 1976) 30(11):1283-1286. PMID 15928553; DOI 10.1097/01.brs.0000164257.69354.bb. [peer-reviewed; abstract]; n = 10. Cadaveric thoracic spines with and without intact rib cage. ([record](https://europepmc.org/abstract/MED/15928553)) Record: bibliographic record and read status.

- `mannen-2018` — Mannen EM, Friis EA, Sis HL, Wong BM, Cadel ES, Anderson DE (2018). The rib cage stiffens the thoracic spine in a cadaveric model with body weight load under dynamic moments. J Mech Behav Biomed Mater 84:258-264. PMID 29852313; DOI 10.1016/j.jmbbm.2018.05.019; PMCID PMC6016553. [peer-reviewed; full-text]; n = 8 intact;7 no-ribs EZS specimens. Whole T1–T12 cadaver spines; 400 N follower load, ±5 Nm dynamic. ([record](https://europepmc.org/abstract/MED/29852313)) Record: bibliographic record and read status.

- `liebsch-2020` — Liebsch C, Wilke HJ (2020). Rib Presence, Anterior Rib Cage Integrity, and Segmental Length Affect the Stability of the Human Thoracic Spine: An in vitro Study. Front Bioeng Biotechnol 8:46. PMID 32117927; DOI 10.3389/fbioe.2020.00046; PMCID PMC7018667. [peer-reviewed; full-text]; n = 8. Intact polysegmental human thoracic specimens; ±5 Nm; anterior ribs then cut. ([record](https://europepmc.org/abstract/MED/32117927)) Record: bibliographic record and read status.

- `petit-2004` — Petit Y, Aubin CE, Labelle H (2004). Patient-specific mechanical properties of a flexible multi-body model of the scoliotic spine. Med Biol Eng Comput 42(1):55-60. PMID 14977223; DOI 10.1007/bf02351011. [peer-reviewed; abstract]; n = 10. Scoliosis bending-film personalisation of per-segment torsion springs. ([record](https://europepmc.org/abstract/MED/14977223)) Record: bibliographic record and read status.

- `prasad-2017` — Prasad K, Bazaka O, Chua M, Rochford M, Fedrick L, Spoor J, Symes R, Tieppo M, Collins C, Cao A, Markwell D, Ostrikov KK, Bazaka K (2017). Metallic Biomaterials: Current Challenges and Opportunities. Materials (Basel) 10(8):E884. PMID 28773240; DOI 10.3390/ma10080884; PMCID PMC5578250. [peer-reviewed; full-text]; n = not given. Review of implant alloys; wrought Ti-6Al-4V,CoCrMo and Stainless steel316L;Table1 states no steel condition. ([record](https://europepmc.org/abstract/MED/28773240)) Record: bibliographic record and read status.

- `little-2013` — Little JP, Izatt MT, Labrom RD, Askin GN, Adam CJ (2013). An FE investigation simulating intra-operative corrective forces applied to correct scoliosis deformity. Scoliosis 8(1):9. PMID 23680391; DOI 10.1186/1748-7161-8-9; PMCID PMC3680303. [peer-reviewed; full-text]; n = 8. AIS FE comparison with1-week standing Cobb; adult material properties. ([record](https://europepmc.org/abstract/MED/23680391)) Record: bibliographic record and read status.

- `ohrt-nissen-2018` — Ohrt-Nissen S, Dahl B, Gehrchen M (2018). Choice of Rods in Surgical Treatment of Adolescent Idiopathic Scoliosis: What Are the Clinical Implications of Biomechanical Properties? - A Review of the Literature. Neurospine 15(2):123-130. PMID 29991241; DOI 10.14245/ns.1836050.025; PMCID PMC6104735. [peer-reviewed; full-text]; n = not given. Review quoting textbook alloy ranges; fatigue cycle count not given. ([record](https://europepmc.org/abstract/MED/29991241)) Record: bibliographic record and read status.

- `jeong-2025` — Jeong BC, Sim O, Lee C (2025). Simulation and validation of spinal construct testing based on ASTM F1717. Front Bioeng Biotechnol 13:1673061. PMID 41383715; DOI 10.3389/fbioe.2025.1673061; PMCID PMC12690298. [peer-reviewed; full-text]; n = not given. FE of ASTM F1717 construct; Ti-6Al-4V ELI; cyclic behaviour not addressed. ([record](https://europepmc.org/abstract/MED/41383715)) Record: bibliographic record and read status.

- `song-2021` — Song M, Sun K, Li Z, Zong J, Tian X, Ma K, Wang S (2021). Stress distribution of different lumbar posterior pedicle screw insertion techniques: a combination study of finite element analysis and biomechanical test. Sci Rep 11(1):12968. PMID 34155224; DOI 10.1038/s41598-021-90686-6; PMCID PMC8217271. [peer-reviewed; full-text]; n = not given. L3–S1 FE models; Ti alloy; 300 N preload plus10 Nm moment. ([record](https://europepmc.org/abstract/MED/34155224)) Record: bibliographic record and read status.

- `lehman-2003` — Lehman RA, Polly DW, Kuklo TR, Cunningham B, Kirk KL, Belmont PJ (2003). Straight-forward versus anatomic trajectory technique of thoracic pedicle screw fixation: a biomechanical analysis. Spine (Phila Pa 1976) 28(18):2058-2065. PMID 14501914; DOI 10.1097/01.brs.0000087743.57439.4f. [peer-reviewed; abstract]; n = 30 vertebrae, paired. Adult thoracic cadaver vertebrae; straight-forward and anatomic trajectories. ([record](https://europepmc.org/abstract/MED/14501914)) Record: bibliographic record and read status.

- `liljenqvist-2001` — Liljenqvist U, Hackenberg L, Link T, Halm H (2001). Pullout strength of pedicle screws versus pedicle and laminar hooks in the thoracic spine. Acta Orthop Belg 67(2):157-163. PMID 11383294. [peer-reviewed; abstract]; n = 9 spines;90 tests. Adult thoracic spines; T4–T8 andT9–T12 screws and hooks. ([record](https://europepmc.org/abstract/MED/11383294)) Record: bibliographic record and read status.

- `soshi-1991` — Soshi S, Shiba R, Kondo H, Murota K (1991). An experimental study on transpedicular screw fixation in relation to osteoporosis of the lumbar spine. Spine (Phila Pa 1976) 16(11):1335-1341. PMID 1750009; DOI 10.1097/00007632-199111000-00015. [peer-reviewed; abstract]; n = not given. Cadaveric lumbar vertebrae, normal and osteoporosis gradesI/II;7.0mm screws. Adulthood not stated in the abstract. ([record](https://europepmc.org/abstract/MED/1750009)) Record: bibliographic record and read status.

- `le-cann-2015` — Le Cann S, Cachon T, Viguier E, Miladi L, Odent T, Rossi JM, Chabrand P (2015). Pedicle Screw Fixation Study in Immature Porcine Spines to Improve Pullout Resistance during Animal Testing. PLoS One 10(10):e0127463. PMID 26451947; DOI 10.1371/journal.pone.0127463; PMCID PMC4599826. [peer-reviewed; full-text]; n = 80 vertebrae. Immature pigs aged90 and140 days; mono-/bicortical, thoracic/lumbar. ([record](https://europepmc.org/abstract/MED/26451947)) Record: bibliographic record and read status.

- `yilmaz-2009` — Yilmaz G, Demirkiran G, Ozkan C, Daglioglu K, Pekmezci M, Alanay A, Yazici M (2009). The effect of dilation of immature pedicles on pullout strength of the screws: Part 2: In vivo study. Spine (Phila Pa 1976) 34(22):2378-2383. PMID 19829250; DOI 10.1097/brs.0b013e3181b0bb9b. [peer-reviewed; abstract]; n = 2 animals with data;3 operated,1 lost. Pigs six weeks old at surgery;pull-out tested three months later after killing;L1–L5. Data from two surviving animals. ([record](https://europepmc.org/abstract/MED/19829250)) Record: bibliographic record and read status.

- `cheng-2000` — Cheng JC, Qin L, Cheung CS, Sher AH, Lee KM, Ng SW, Guo X (2000). Generalized low areal and volumetric bone mineral density in adolescent idiopathic scoliosis. J Bone Miner Res 15(8):1587-1595. PMID 10934658; DOI 10.1359/jbmr.2000.15.8.1587. [peer-reviewed; abstract]; n = 75 AIS;94 controls. AIS girls aged12–14;75AIS and94controls;38%aBMD/36%vBMD below−1SD. ([record](https://europepmc.org/abstract/MED/10934658)) Record: bibliographic record and read status.

- `yip-2016` — Yip BHK, Yu FWP, Wang Z, Hung VWY, Lam TP, Ng BKW, Zhu F, Cheng JCY (2016). Prognostic Value of Bone Mineral Density on Curve Progression: A Longitudinal Cohort Study of 513 Girls with Adolescent Idiopathic Scoliosis. Sci Rep 6:39220. PMID 27991528; DOI 10.1038/srep39220; PMCID PMC5171643. [peer-reviewed; full-text]; n = 513;HR-pQCT subgroup90. AIS girls, mean age 13.15, follow-up4.6 years; 169/513 osteopenic. ([record](https://europepmc.org/abstract/MED/27991528)) Record: bibliographic record and read status.

- `winter-via-keenan-2015` — Keenan BE, Pettet GJ, Izatt MT, Askin GN, Labrom RD, Pearcy MJ, Adam C (2015). Gravity-induced coronal plane joint moments in adolescent idiopathic scoliosis. Scoliosis 10:35. PMID 26681978; DOI 10.1186/s13013-015-0060-9; PMCID PMC4682283. [peer-reviewed; full-text]; n = 8 male cadavers (Winter);20 girls (Keenan). Winter cadavers aged61–83; fractions borrowed in AIS girls with Lenke1 curves>40°. ([record](https://europepmc.org/abstract/MED/26681978)) Record: bibliographic record and read status.

- `pearsall-1996` — Pearsall DJ, Reid JG, Livingston LA (1996). Segmental inertial parameters of the human trunk as determined from computed tomography. Ann Biomed Eng 24(2):198-210. PMID 8678352; DOI 10.1007/bf02667349. [peer-reviewed; abstract]; n = 4. Adults, MeSH middle-aged/aged; CT trunk by vertebral level. ([record](https://europepmc.org/abstract/MED/8678352)) Record: bibliographic record and read status.

- `keenan-2014` — Keenan BE, Izatt MT, Askin GN, Labrom RD, Pettet GJ, Pearcy MJ, Adam CJ (2014). Segmental torso masses in adolescent idiopathic scoliosis. Clin Biomech (Bristol) 29(7):773-779. PMID 24973826; DOI 10.1016/j.clinbiomech.2014.06.002. [peer-reviewed; abstract]; n = 20. AIS girls, age 15.0±2.7, Cobb52±5.9°, weight58.2±11.6 kg; CT torso slices. ([record](https://europepmc.org/abstract/MED/24973826)) Record: bibliographic record and read status.

- `gangnet-2003` — Gangnet N, Pomero V, Dumas R, Skalli W, Vital JM (2003). Variability of the spine and pelvis location with respect to the gravity line: a three-dimensional stereoradiographic study using a force platform. Surg Radiol Anat 25(5-6):424-433. PMID 13680185; DOI 10.1007/s00276-003-0154-6. [peer-reviewed; abstract]; n = 34. 34 healthy upright standing subjects, taken here as adults;the abstract supplies no ages. ([record](https://europepmc.org/abstract/MED/13680185)) Record: bibliographic record and read status.

- `rohlmann-2002` — Rohlmann A, Graichen F, Bergmann G (2002). Loads on an internal spinal fixation device during physical therapy. Phys Ther 82(1):44-52. PMID 11784277; DOI 10.1093/ptj/82.1.44. [peer-reviewed; abstract]; n = 10. Adults with telemetered posterior internal fixators; fractures/degenerative disease, not scoliosis. ([record](https://europepmc.org/abstract/MED/11784277)) Record: bibliographic record and read status.

- `rohlmann-2014-gait` — Rohlmann A, Dreischarf M, Zander T, Graichen F, Bergmann G (2014). Loads on a vertebral body replacement during locomotion measured in vivo. Gait Posture 39(2):750-755. PMID 24211089; DOI 10.1016/j.gaitpost.2013.10.010. [peer-reviewed; abstract]; n = 5. Adults with instrumented vertebral body replacements; walking/stairs. ([record](https://europepmc.org/abstract/MED/24211089)) Record: bibliographic record and read status.

- `wilke-1999` — Wilke HJ, Neef P, Caimi M, Hoogland T, Claes LE (1999). New in vivo measurements of pressures in the intervertebral disc in daily life. Spine (Phila Pa 1976) 24(8):755-762. PMID 10222525; DOI 10.1097/00007632-199904150-00005. [peer-reviewed; abstract]; n = 1. Man aged45, 70 kg; intradiscal pressure atL4–L5. ([record](https://europepmc.org/abstract/MED/10222525)) Record: bibliographic record and read status.

- `schmid-2020-ais` — Schmid S, Burkhart KA, Allaire BT, Grindle D, Bassani T, Galbusera F, Anderson DE (2020). Spinal Compressive Forces in Adolescent Idiopathic Scoliosis With and Without Carrying Loads: A Musculoskeletal Modeling Study. Front Bioeng Biotechnol 8:159. PMID 32195239; DOI 10.3389/fbioe.2020.00159; PMCID PMC7062648. [peer-reviewed; full-text]; n = 24. AIS aged9–17(mean13.8), 29–63 kg, Cobb13.6–28.3°; musculoskeletal models. ([record](https://europepmc.org/abstract/MED/32195239)) Record: bibliographic record and read status.

- `tudor-locke-2008` — Tudor-Locke C, Hatano Y, Pangrazi RP, Kang M (2008). Revisiting "how many steps are enough?". Med Sci Sports Exerc 40(7 Suppl):S537-43. PMID 18562971; DOI 10.1249/mss.0b013e31817c7133. [peer-reviewed; abstract]; n = not given. Adult step bands; youth cut points not established. ([record](https://europepmc.org/abstract/MED/18562971)) Record: bibliographic record and read status.

- `tudor-locke-2011` — Tudor-Locke C, Craig CL, Beets MW, Belton S, Cardon GM, Duncan S, Hatano Y, Lubans DR, Olds TS, Raustorp A, Rowe DA, Spence JC, Tanaka S, Blair SN (2011). How many steps/day are enough? for children and adolescents. Int J Behav Nutr Phys Act 8:78. PMID 21798014; DOI 10.1186/1479-5868-8-78; PMCID PMC3166269. [peer-reviewed; full-text]; n = not given. Review of children/adolescent steps per day. ([record](https://europepmc.org/abstract/MED/21798014)) Record: bibliographic record and read status.

- `vieweg-2023` — Vieweg U, Keck J, Krüger S, Arabmotlagh M, Rauschmann M, Schilling C (2023). Biomechanical comparison of different rod-to-rod connectors to a conventional titanium- and cobalt chromium posterior spinal fixation system. Brain Spine 3:101708. PMID 36685708; DOI 10.1016/j.bas.2022.101708; PMCID PMC9845396. [peer-reviewed; full-text]; n = 6 per dynamic compression-bending group. F1717-21 vertebrectomy constructs, 76 mm active length, 5.5 mm rods, 4.5×45 mm screws. ([record](https://europepmc.org/abstract/MED/36685708)) Record: bibliographic record and read status.

- `slivka-2013` — Slivka MA, Fan YK, Eck JC (2013). The Effect of Contouring on Fatigue Strength of Spinal Rods: Is it Okay to Re-bend and Which Materials Are Best? Spine Deform 1(6):395-400. PMID 27927364; DOI 10.1016/j.jspd.2013.08.004. [peer-reviewed; abstract]; n = not given. Contoured/re-bent4.5 mm Ti, stainless-steel, CoCr rod constructs; CoCrscrews failed first. ([record](https://europepmc.org/abstract/MED/27927364)) Record: bibliographic record and read status.

- `carrion-2016` — Carrion PE, Shamsaei N (2016). Strain-based fatigue data for Ti-6Al-4V ELI under fully-reversed and mean strain loads. Data Brief 7:12-15. PMID 26952022; DOI 10.1016/j.dib.2016.02.014; PMCID PMC4761652. [peer-reviewed; full-text]; n = 38 specimens. Annealed polished wroughtTi-6Al-4VELI coupons, strain-controlled. ([record](https://europepmc.org/abstract/MED/26952022)) Record: bibliographic record and read status.

- `abe-2015` — Abe Y, Ito M, Abumi K, Sudo H, Salmingo R, Tadano S (2015). Scoliosis corrective force estimation from the implanted rod deformation using 3D-FEM analysis. Scoliosis 10(Suppl 2):S2. PMID 25810754; DOI 10.1186/1748-7161-10-s2-s2; PMCID PMC4331738. [peer-reviewed; full-text]; n = 20. AIS patients, mean age14.8; 6 mm Ti6Al7Nb rods, elasto-plastic inverse FE. ([record](https://europepmc.org/abstract/MED/25810754)) Record: bibliographic record only.

- `cidambi-2012` — Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO (2012). Postoperative changes in spinal rod contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) 37(18):1566-1572. PMID 22426445; DOI 10.1097/brs.0b013e318252ccbe. [peer-reviewed; abstract]; n = 27. Patients with 5.5 mm ultra-high-strength steel rods; disease label not restated here. ([record](https://europepmc.org/abstract/MED/22426445)) Record: bibliographic record only.

- `karami-2015` — Karami KJ, Buckenmeyer LE, Kiapour AM, Kelkar PS, Goel VK, Demetropoulos CK, Soo TM (2015). Biomechanical evaluation of the pedicle screw insertion depth effect on screw stability under cyclic loading and subsequent pullout. J Spinal Disord Tech 28(3):E133-9. PMID 25310387; DOI 10.1097/bsd.0000000000000178. [peer-reviewed; abstract]; n = 10 spines;100 pedicles. Osteoporotic lumbar spines; pull-out after cyclic loading. Age not recorded. ([record](https://europepmc.org/abstract/MED/25310387)) Record: bibliographic record only.

- `sairyo-1998` — Sairyo K, Scifert J, Goel VK, Grosland N, Grobler LJ (1998). Neurocentral synchondrosis fracture in immature spines associated with pedicle screw type fixation devices. J Spinal Disord 11(2):142-145. PMID 9588471; DOI 10.1097/00002517-199804000-00009. [peer-reviewed; abstract]; n = 10 vertebrae. Immature calf; 6 and7 mm screws. ([record](https://europepmc.org/abstract/MED/9588471)) Record: bibliographic record only.

- `kang-2014` — Kang DG, Lehman RA Jr, Bevevino AJ, Gaume RE, Purcell RL, Dmitriev AE, Lenke LG (2014). Pedicle screw "hubbing" in the immature thoracic spine: a biomechanical and micro-computed tomography evaluation. J Pediatr Orthop 34(7):703-709. PMID 24590340; DOI 10.1097/bpo.0000000000000166. [peer-reviewed; abstract]; n = 22 specimens. Immature calf thoracic vertebrae; hubbed versus non-hubbed. ([record](https://europepmc.org/abstract/MED/24590340)) Record: bibliographic record only.

- `tsai-2009` — Tsai WC, Chen PQ, Lu TW, Wu SS, Shih KS, Lin SC (2009). Comparison and prediction of pullout strength of conical and cylindrical pedicle screws within synthetic bone. BMC Musculoskelet Disord 10:44. PMID 19402917; DOI 10.1186/1471-2474-10-44; PMCID PMC2694760. [peer-reviewed; full-text]; n = 6 commercial screws;6 trials each. ASTM F1839 polyurethane foam; 30 mm engaged thread. ([record](https://europepmc.org/abstract/MED/19402917)) Record: bibliographic record only.

- `kueny-2014` — Kueny RA, Kolb JP, Lehmann W, Püschel K, Morlock MM, Huber G (2014). Influence of the screw augmentation technique and a diameter increase on pedicle screw fixation in the osteoporotic spine: pullout versus fatigue testing. Eur Spine J 23(10):2196-2202. PMID 25082759; DOI 10.1007/s00586-014-3476-7. [peer-reviewed; abstract]; n = 39 vertebrae. Osteoporotic lumbar vertebrae; diameter increment comparison. Age not recorded. ([record](https://europepmc.org/abstract/MED/25082759)) Record: bibliographic record only.

- `sariyilmaz-2021` — Sariyilmaz K, Ozkunt O, Can Gemalmaz H, Cingoz T, Pehlivanoglu T, Aksoy T, Kaya O, Baydogan M, Dikici F (2021). Direct vertebral rotation significantly decreases the pullout strength of the pedicle screw: a biomechanical study in adult cadavers. J Pediatr Orthop B 30(3):230-234. PMID 32453125; DOI 10.1097/bpb.0000000000000751. [peer-reviewed; abstract]; n = 30 pedicles. Adult cadaver thoracic; 5.5 mm polyaxial screws; simulated direct vertebral rotation. ([record](https://europepmc.org/abstract/MED/32453125)) Record: bibliographic record only.

- `weiser-2017` — Weiser L, Huber G, Sellenschloh K, Viezens L, Püschel K, Morlock MM, Lehmann W (2017). Insufficient stability of pedicle screws in osteoporotic vertebrae: biomechanical correlation of bone mineral density and pedicle screw fixation strength. Eur Spine J 26(11):2891-2897. PMID 28391382; DOI 10.1007/s00586-017-5091-x. [peer-reviewed; abstract]; n = 21 T12 vertebrae. QCT bone-density groups; cyclic screw failure. Age not recorded. ([record](https://europepmc.org/abstract/MED/28391382)) Record: bibliographic record only.

- `santoni-2009` — Santoni BG, Hynes RA, McGilvray KC, Rodriguez-Canessa G, Lyons AS, Henson MA, Womack WJ, Puttlitz CM (2009). Cortical bone trajectory for lumbar pedicle screws. Spine J 9(5):366-373. PMID 18790684; DOI 10.1016/j.spinee.2008.07.008. [peer-reviewed; abstract]; n = not given. lumbar cortical versus traditional screw trajectory. Age not recorded. ([record](https://europepmc.org/abstract/MED/18790684)) Record: bibliographic record only.

- `keenan-2015` — Keenan BE, Pettet GJ, Izatt MT, Askin GN, Labrom RD, Pearcy MJ, Adam C (2015). Gravity-induced coronal plane joint moments in adolescent idiopathic scoliosis. Scoliosis 10:35. PMID 26681978; DOI 10.1186/s13013-015-0060-9; PMCID PMC4682283. [peer-reviewed; full-text]; n = 20 girls. AIS Lenke1 curves over40°, mean mass57.5kg; gravity moment model. ([record](https://europepmc.org/abstract/MED/26681978)) Record: bibliographic record only.

- `rohlmann-2008-standing` — Rohlmann A, Graichen F, Kayser R, Bender A, Bergmann G (2008). Loads on a telemeterized vertebral body replacement measured in two patients. Spine (Phila Pa 1976) 33(11):1170-1179. PMID 18469689; DOI 10.1097/brs.0b013e3181722d52. [peer-reviewed; abstract]; n = 2. Adults with instrumented vertebral body replacements, one month after surgery. ([record](https://europepmc.org/abstract/MED/18469689)) Record: bibliographic record only.

- `rohlmann-2013-lifting` — Rohlmann A, Zander T, Graichen F, Bergmann G (2013). Lifting up and laying down a weight causes high spinal loads. J Biomech 46(3):511-514. PMID 23141957; DOI 10.1016/j.jbiomech.2012.10.022. [peer-reviewed; abstract]; n = not given. Adults with instrumented vertebral body replacements; lifting/setting down0.01–3 kg. ([record](https://europepmc.org/abstract/MED/23141957)) Record: bibliographic record only.

- `rohlmann-2014-activities` — Rohlmann A, Pohl D, Bender A, Graichen F, Dymke J, Schmidt H, Bergmann G (2014). Activities of everyday life with high spinal loads. PLoS One 9(5):e98510. PMID 24866883; DOI 10.1371/journal.pone.0098510; PMCID PMC4035320. [peer-reviewed; full-text]; n = 5. Adults aged62–71, 60–74kg; L1/L3vertebral body replacements, posterior fixator shares load. ([record](https://europepmc.org/abstract/MED/24866883)) Record: bibliographic record only.

- `rohlmann-2008-month-one` — Rohlmann A, Graichen F, Bender A, Kayser R, Bergmann G (2008). Loads on a telemeterized vertebral body replacement measured in three patients within the first postoperative month. Clin Biomech (Bristol) 23(2):147-158. PMID 17983694; DOI 10.1016/j.clinbiomech.2007.09.011. [peer-reviewed; abstract]; n = 3. Adults with vertebral body replacements; first postoperative month. ([record](https://europepmc.org/abstract/MED/17983694)) Record: bibliographic record only.

- `damm-2017` — Damm P, Kutzner I, Bergmann G, Rohlmann A, Schmidt H (2017). Comparison of in vivo measured loads in knee, hip and spinal implants during level walking. J Biomech 51:128-132. PMID 27914627; DOI 10.1016/j.jbiomech.2016.11.060. [peer-reviewed; abstract]; n = 5 VBR patients. Adults with instrumented vertebral body replacements; mean walking peak. ([record](https://europepmc.org/abstract/MED/27914627)) Record: bibliographic record only.

- `rohlmann-1997-walker` — Rohlmann A, Bergmann G, Graichen F (1997). Loads on an internal spinal fixation device during walking. J Biomech 30(1):41-47. PMID 8970923; DOI 10.1016/s0021-9290(96)00103-0. [peer-reviewed; abstract]; n = 2. Adults with telemetered posterior internal fixators; walker use. ([record](https://europepmc.org/abstract/MED/8970923)) Record: bibliographic record only.

- `rohlmann-2015-flexion` — Dreischarf M, Albiol L, Zander T, Arshad R, Graichen F, Bergmann G, Schmidt H, Rohlmann A (2015). In vivo implant forces acting on a vertebral body replacement during upper body flexion. J Biomech 48(4):560-565. PMID 25640900; DOI 10.1016/j.jbiomech.2015.01.007. [peer-reviewed; abstract]; n = not given. Adult vertebral body replacement; upper-body flexion and return to upright. ([record](https://europepmc.org/abstract/MED/25640900)) Record: bibliographic record only.

- `rohlmann-2014-carrying` — Rohlmann A, Zander T, Graichen F, Schmidt H, Bergmann G (2014). How does the way a weight is carried affect spinal loads? Ergonomics 57(2):262-270. PMID 24559120; DOI 10.1080/00140139.2014.887789. [peer-reviewed; abstract]; n = not given. Adult vertebral body replacement; carrying10 kg backpack or weights. ([record](https://europepmc.org/abstract/MED/24559120)) Record: bibliographic record only.

- `rohlmann-2014-cycling` — Rohlmann A, Zander T, Graichen F, Schmidt H, Bergmann G (2014). Spinal loads during cycling on an ergometer. PLoS One 9(4):e95497. PMID 24743577; DOI 10.1371/journal.pone.0095497; PMCID PMC3990685. [peer-reviewed; abstract]; n = not given. Adult vertebral body replacement; ergometer cycling. ([record](https://europepmc.org/abstract/MED/24743577)) Record: bibliographic record only.

- `rohlmann-2014-vibration` — Rohlmann A, Schmidt H, Gast U, Kutzner I, Damm P, Bergmann G (2014). In vivo measurements of the effect of whole body vibration on spinal loads. Eur Spine J 23(3):666-672. PMID 24201510; DOI 10.1007/s00586-013-3087-8; PMCID PMC3940795. [peer-reviewed; abstract]; n = not given. Adult vertebral body replacement; whole-body vibration versus same posture. ([record](https://europepmc.org/abstract/MED/24201510)) Record: bibliographic record only.

- `rohlmann-2013-orthosis` — Rohlmann A, Zander T, Graichen F, Bergmann G (2013). Effect of an orthosis on the loads acting on a vertebral body replacement. Clin Biomech (Bristol) 28(5):490-494. PMID 23623494; DOI 10.1016/j.clinbiomech.2013.03.010. [peer-reviewed; abstract]; n = not given. Adult vertebral body replacement; orthosis comparison over26 activities. ([record](https://europepmc.org/abstract/MED/23623494)) Record: bibliographic record only.
