# The typical spine

`typical_spine.py` writes `engine/joint-fit/typical-spine/typical-spine.json`,
the typical spine the fit falls back on when a case has only a front film.
It is run by hand when a source or the bone shapes change; the engine
compiles the file in and never fetches it.

## What it is for

A front film shows where each vertebra sits from side to side, how it
tilts, how wide and how tall it is. It does not show how far each vertebra
leans forward or back, how deep its body is, how its body is wedged from
front to back, or how far it is twisted: those barely move a body's corners
on a front film. For each of them the file holds a
typical value and how widely it varies between people:

| Part | Holds |
|---|---|
| `bodies` | per level, C7 to L5: half-width, depth over width, side wedge |
| `sideProfiles` | per curve size: the forward lean of each of the 18 levels |
| `stackingSpread` | how far a centre may sit off the line its neighbours' leans give |
| `twistAtApex` | per curve size: the twist of the curve's apex vertebra |

Every value is typical, not measured, and the fit reports it with the
spread given here. Signs follow `docs/conventions/signs-and-wedges.md`:
forward lean positive tips the column forward going up; side wedge
positive when the front is shorter; twist positive turns the front toward
the patient's left (the file gives its size; the fit gives its direction).

## Sources

| Key | Source | Read | What it gave |
|---|---|---|---|
| `bodyparts3d` | BodyParts3D 4.0, The Database Center for Life Science, CC Attribution 4.0 International (see `README.md`) | the bone file | the per-level shape of the side profile, widths, depth over width, each body's side wedge, the stacking spread |
| `burkus2018` | Burkus M, Schlégl ÁT, O'Sullivan I, Márkus I, Vermes C, Tunyogi-Csapó M. Sagittal plane assessment of spino-pelvic complex in a Central European population with adolescent idiopathic scoliosis: a case control study. Scoliosis Spinal Disord 2018;13:10. DOI 10.1186/s13013-018-0156-0 | full text | 69 people aged 12 to 26 without scoliosis: kyphosis T5-T12 32.9 (SD 15.0) degrees, lordosis L1-L5 46.0 (9.1); 458 with scoliosis: lordosis L1-L5 46.4 (13.2), not different (p = 0.83) |
| `sullivan2020` | Sullivan TB, Bastrom TP, Bartley CE, Dolan LA, Weinstein SL, Newton PO. More severe thoracic idiopathic scoliosis is associated with a greater three-dimensional loss of thoracic kyphosis. Spine Deform 2020;8(6):1205-1211. DOI 10.1007/s43390-020-00149-7 | abstract only | 3,032 patients: three-dimensional kyphosis T5-T12 13 (SD 10) degrees for curves of 20 to 45 degrees, 5 (12) for curves over 45 |
| `newton2015` | Newton PO, Fujimori T, Doan J, Reighard FG, Bastrom TP, Misaghi A. Defining the "three-dimensional sagittal plane" in thoracic adolescent idiopathic scoliosis. J Bone Joint Surg Am 2015;97(20):1694-1701. DOI 10.2106/JBJS.O.00148 | abstract only | 120 patients with a mean curve of 55 (SD 10) degrees had three-dimensional kyphosis T5-T12 of 6 (14) degrees |
| `macthiong2007` | Mac-Thiong JM, Labelle H, Berthonnaud E, Betz RR, Roussouly P. Sagittal spinopelvic balance in normal children and adolescents. Eur Spine J 2007;16(2):227-234. DOI 10.1007/s00586-005-0013-8 | abstract only | 341 children aged 3 to 18: thoracic tilt −3.1 (SD 5.2) degrees, lumbar tilt −7.3 (5.2) |
| `jozsef2022` | József K, Schlégl ÁT, Burkus M, Márkus I, O'Sullivan I, Than P, Tunyogi-Csapó M. Maximal axial vertebral rotation in adolescent idiopathic scoliosis: is the apical vertebra the most rotated? Global Spine J 2022;12(2):244-248. DOI 10.1177/2192568220948830 | full text | 332 patients, curves 10 to 122 degrees (mean 37.7): apical twist by curve band, table 4 |
| `pan2021` | Pan CL, Zhang BY, Zhu YH, Ma YH, Li MF, Wang X, Yang F, Li YQ, Zhu YH. Morphologic analysis of Chinese lumbar endplate by three-dimensional computed tomography reconstructions for helping design lumbar disc prosthesis. Medicine (Baltimore) 2021;100(6):e24583. DOI 10.1097/MD.0000000000024583 | full text | 150 adults: endplate depth over width, mean and SD, T12 to L5, men and women (table 2) |
| `busscher2010` | Busscher I, Ploegmakers JJ, Verkerke GJ, Veldhuizen AG. Comparative anatomical dimensions of the complete human and porcine spine. Eur Spine J 2010;19(7):1104-1114. DOI 10.1007/s00586-010-1326-9 | full text | six adult spines: SD of each body's front and back height, C7 to L5 (table 3) |

Every number the script uses is in one table at the top of
`typical_spine.py`, with its unit, its source key and whether the paper or
only its abstract was read. The full texts of Sullivan 2020 and Newton 2015
were behind a paywall, and the open copy of Mac-Thiong 2007 holds only its
abstract, which does not say how its tilts are measured.

## How the script builds it

### 1. Bodies

- **Half-width**: the bone's own `halfWidth`.
- **Depth over width**: from the two endplates measured in the bone's
  `body` (see `README.md`, "The body's endplates"): the top and bottom
  half-depths added together, over the top and bottom half-widths added
  together. Its spread is a fixed fraction of it. Pan 2021 gives depth over
  width with its SD at six levels for men and women; SD over mean ranges
  from 0.056 (men, T12) to 0.128 (men, L1), and the root mean square of the
  twelve is **0.0964**. So L5, 0.829, has SD 0.829 × 0.0964 = 0.080. The
  same fraction is used for C7 to T11, where no open source gave a spread.
- **Side wedge**: the bone's own `body.wedgeSide`, the angle between its
  two measured endplates seen from the side, in degrees to a tenth: from
  −10.1 (L5) to +5.6 (C7). The spread comes from Busscher 2010: the SDs of a body's front and back
  heights, taken as independent, across the bone's depth. At T7:
  √(1.1² + 1.7²) = 2.02 mm over 29.98 mm is 3.9 degrees. Single levels
  range from 3.0 (C7, T12) to 8.0 degrees (T5) on six spines, so the 18
  are pooled by root mean square: **4.9 degrees** at every level. Front and
  back heights of one body rise and fall together, so this is wider than
  the true spread.

### 2. Side profiles

**Definitions.** A level's forward lean is the lean of its body's axis,
read from the bone's `referenceAxes`. Kyphosis is measured from T5's upper
endplate to T12's lower endplate and lordosis from L1's upper endplate to
L5's lower endplate, as both cohorts measure them. Each endplate leans by
half its body's side wedge, so

```
kyphosis = lean(T5) − lean(T12) + (wedge(T5) + wedge(T12)) / 2
lordosis = lean(L5) − lean(L1) − (wedge(L1) + wedge(L5)) / 2
```

with the side wedges of the bodies above. The bones as they stood have
kyphosis 23.0 degrees between body axes, 24.3 endplate to endplate, and
lordosis 31.5 between body axes, 37.8 endplate to endplate.

**Cohort values by curve size.** Kyphosis is taken through three points
and joined by straight lines:

| Curve | Kyphosis T5-T12 | SD | From |
|---|---|---|---|
| 0 | 32.9 | 15.0 | Burkus 2018, people without scoliosis |
| 32.5 | 13 | 10 | Sullivan 2020, curves 20 to 45, placed at the band's middle |
| 55 | 5 | 12 | Sullivan 2020, curves over 45, placed at Newton 2015's mean curve |

Beyond 55 degrees it is held at 5. Lordosis is 46.0 degrees at every curve
size (Burkus 2018 found no difference with scoliosis); its SD is 9.1 for a
straight spine and 13.2, the scoliosis group's, for every curved row.
Burkus measured kyphosis on the side view; Sullivan's and Newton's are
three-dimensional. With no curve there is no twist to make the two differ.

**Scaling.** The bones' lean steps between neighbours keep their shape.
The steps from T5 to T12 are multiplied by one factor so kyphosis equals
the cohort value, and the steps from L1 to L5 by another so lordosis does;
the steps above T5 and between T12 and L1 are left as they are. At curve 0
the thoracic factor is (32.9 − 1.3) / 23.0 = 1.374 and the lumbar factor
(46.0 − 6.3) / 31.5 = 1.260. Adding the steps up from L5 gives a
continuous profile.

**Levelling.** The bodies are stacked from L5 upward: each centre sits
above the one below by the bones' own centre-to-centre distance (two half
heights and the disc gap between them, 17.8 to 35.4 mm), along the mean of
the two leans. One angle is then added to every lean so the line from L5's
centre to T1's is upright:

```
offset = −atan2(Σ rise × sin(mean lean), Σ rise × cos(mean lean))     pairs T1-T2 to L4-L5
```

Stacked this way, the bones' own leans put T1 0.9 degrees forward of
upright.

**Result.** Recomputed from the leans and wedges as written, which are
rounded to 0.1 degree:

| Curve | Thoracic factor | Kyphosis T5-T12 | Lordosis L1-L5 | Kyphosis T1-T12 | Kyphosis T4-T12 | T1-L5 line |
|---|---|---|---|---|---|---|
| 0 | 1.374 | 32.9 | 46.0 | 48.4 | 36.6 | −0.01 |
| 20 | 0.841 | 20.7 | 46.0 | 36.3 | 24.4 | −0.00 |
| 32 | 0.522 | 13.3 | 46.0 | 28.9 | 17.0 | −0.01 |
| 45 | 0.315 | 8.6 | 46.0 | 24.2 | 12.3 | −0.01 |
| 55, 60, 90 | 0.161 | 5.0 | 46.0 | 20.6 | 8.7 | −0.01 |

For comparison, Burkus 2018's group without scoliosis had T1-T12 43.4
and T4-T12 37.7 degrees. Because the line from T1 to L5 is kept upright,
flattening the kyphosis moves the lumbar leans forward: L4 leans 21.7
degrees at curve 0 and 30.6 at curve 55.

### 3. Spread of each lean

A person's leans differ from the row's by their own kyphosis, their own
lordosis and their whole trunk's lean. The script finds how much each
level's lean moves per degree of kyphosis (`gK`) and of lordosis (`gL`),
levelling included, and adds the three as independent:

```
sd = √( (gK × SD of kyphosis)² + (gL × SD of lordosis)² + 5.2² )
```

5.2 degrees is the larger of the two tilt spreads in Mac-Thiong 2007,
taken as the spread of the trunk's lean as a whole. At curve 0, T1 has
gK 0.684 and gL −0.186, so √(10.26² + 1.69² + 5.2²) = 11.6 degrees; L4
has gK −0.316 and gL 0.830, so √(4.74² + 7.55² + 5.2²) = 10.3. Spreads run
from 5.7 to 12.9 degrees.

### 4. Stacking spread

For each of the 17 neighbouring pairs in the bones as they stood, the
script measures how far the upper centre sits forward of the line drawn
up from the lower centre along the pair's mean lean, as a fraction of the
rise between them. The fractions run from −0.168 (T1-T2) to +0.049
(L4-L5); their root mean square is **0.068**. The leans in the bone file
are read in half-degree steps, which accounts for at most 0.004 of this.

### 5. Twist at the apex

From József 2022, table 4 (absolute values):

| Curve band | Placed at | Curves | Apical twist | SD |
|---|---|---|---|---|
| none | 0 | | 0 | 4.55 |
| 10 to 20 | 15 | 76 | 6.70 | 4.55 |
| 21 to 35 | 28 | 118 | 9.34 | 5.14 |
| 36 to 40 | 38 | 19 | 16.36 | 7.30 |
| 41 to 50 | 45.5 | 43 | 15.67 | 5.07 |
| 51 to 55 | 53 | 10 | 15.84 | 5.77 |
| 56 and over | 73.7 | 66 | 21.27 | 13.05 |

Each closed band sits at its middle (the study's smallest curve was 10
degrees). The open top band sits where the study's mean curve puts it:
(37.7 × 332 − 7,652.5) / 66 = 73.7, where 7,652.5 is the sum of each
other band's middle times its count. No twist at no curve is an
assumption; its spread is the smallest band's. The band means do not rise
steadily (36 to 40 holds 19 curves, 51 to 55 holds 10) and are written as
published.

## What is not known

- No paper predicts a child's side profile from a front film. The side
  profile is a cohort's typical one for the curve size, and the spread is
  wide: a person's kyphosis can differ from it by 20 degrees or more.
- The kyphosis for curves over 20 degrees comes from abstracts only, and
  where each band sits on the curve axis is placed, not reported. Between
  0 and 32.5 degrees, and beyond 55, the values are joined or held by
  assumption.
- The relation between curve size and twist leaves about two thirds of
  the variance unexplained: in József 2022 the spread within the curve
  bands (pooled SD 7.4 degrees) is 0.66 of the whole spread's variance
  (SD 9.15). The most twisted vertebra is the apex in only 40.4% of major
  curves.
- The per-level shape is one adult man's posture; nothing here is
  specific to children. The depth spread is from adult lumbar endplates
  and is used at every level. The side wedge spread is an upper bound from
  six adult spines.
- Kyphosis, lordosis and trunk lean are treated as independent, though
  Mac-Thiong 2007 found neighbouring regions related. Keeping the line from
  T1 to L5 upright is a choice, not a cohort finding.

## Running it

```
python3 bone-prep/typical_spine.py
```

It needs Python 3.10 or later and nothing else, reads
`app/src/spine-3d/bones/vertebrae.json`, writes
`engine/joint-fit/typical-spine/typical-spine.json`, and prints each
row's kyphosis, lordosis and the lean of the line from T1 to L5,
recomputed from what it wrote. The same inputs always give the same bytes.
