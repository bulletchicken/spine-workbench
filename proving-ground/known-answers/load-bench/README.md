# Load-bench known answers

Five closed-form fixtures for joint rotations, rod statics, beam compliance,
springs in series and Goodman conversion. Every expected number is derived
below from the formula, by hand from the conventions alone, never by running
the engine. Python was used only as a calculator. Non-terminating values
carry at least 16 significant digits; integers and terminating decimals are
exact. All numerical inputs are made-up test choices [own inference], not
published material properties, patient measurements or evidence tables.

Each JSON file contains its own units, axes, sign convention, inputs,
expected outputs and tolerance. Nothing reads a material or stiffness table.
`cases[].input` supplies the case; `cases[].expect` supplies its answer.
These are primitive mechanics fixtures, not `BenchInput` records. Later
solver and load tests can read them without constructing a whole spine.

## Units, frames and comparisons

Positions and lengths are mm, forces N, moments N·mm and angles radians.
Moduli and stresses are N/mm²; 1 N/mm² = 1 MPa. The right-handed Cartesian
axes follow the patient-frame directions: x anterior, y patient-left,
z superior, so x cross y = z. Each test uses its stated synthetic origin.
Vectors are `[x, y, z]`; a rotation matrix is an array of rows multiplying
a column vector, an active rotation from moving axes into fixed axes.
Positive moments and rotations use the right-hand rule. Units and the
relevant signs are repeated inside every JSON, including the scalar cases.

Every file states the componentwise finite-number comparison:

```text
|actual - expected| <= 1e-12 + 1e-10 |expected|
```

The relative tolerance is dimensionless; the absolute tolerance is in the
unit of the compared scalar (including each vector or matrix component).
For zero entries only the absolute term remains. Status strings, `null`,
case counts and vector/matrix dimensions must match exactly. These are
closed forms; the tolerance covers floating-point arithmetic and decimal
rounding, not a modelling allowance. Each JSON gives its particular reason.
All finite actual values must be finite before making a comparison.

## 1. `one-joint-at-large-angles.json`

One end is fixed. The other has just one rotational degree of freedom
about the stated unit axis a, with identity rest rotation, stiffness
k = 10000 N·mm/radian and moment vector M a. Translation and the other two
rotations are constrained. The spring resists with moment −kθ a.

The rotation is `R = exp(θ [a]cross)`. Along this one-axis path the log
rotation vector is exactly θ a, so the spring energy is `U = kθ²/2`.
Equilibrium under a moment about that same axis is:

```text
d(U - Mθ)/dθ = kθ - M = 0
θ = M/k
```

There is no small-angle approximation. The same-axis restriction matters:
rotations about one fixed axis commute and applied work is M dθ. For an
off-axis moment the one-degree-of-freedom problem is different; for a
general three-dimensional spring, log-map derivatives and coupled rotations
must be included. Componentwise division by k is not this derivation.
All cases lie below π radians, so the principal log is unambiguous. Large
here includes 2 radians, without linearising sine or cosine; a rotation
matrix and principal log alone do not distinguish multiple full turns.

For each of x, y and z the scalar calculation is identical:

| M (N·mm) | k (N·mm/radian) | θ = M/k (radians) |
|---|---|---|
| 1000 | 10000 | 0.1 |
| 8000 | 10000 | 0.8 |
| 20000 | 10000 | 2 |

Thus the expected rotation vector has θ in the chosen axis component and
zero in its other two components. To obtain every matrix entry, put
`c = cos θ` and `s = sin θ` in the following matrices:

```text
Rx = [ 1  0  0 ]    Ry = [ c  0  s ]    Rz = [ c -s  0 ]
     [ 0  c -s ]         [ 0  1  0 ]         [ s  c  0 ]
     [ 0  s  c ]         [-s  0  c ]         [ 0  0  1 ]
```

The fixed-axis entry is 1; the other fixed-axis row/column entries are 0.
These signs give Rx z = (0, −s, c), Ry z = (s, 0, c) and
Rz x = (c, s, 0), checking all three rotation directions.

| θ | s | c |
|---|---|---|
| 0.1 | 0.09983341664682815230681 | 0.99500416527802576609556 |
| 0.8 | 0.71735609089952276162717 | 0.69670670934716542092075 |
| 2 | 0.90929742682568169539602 | −0.41614683654714238699757 |

For a calculator without radian trigonometry, these values follow from
`sin θ = θ − θ³/3! + θ⁵/5! − ...` and
`cos θ = 1 − θ²/2! + θ⁴/4! − ...`. For θ = 2, retaining sine through
degree 29 and cosine through degree 28 bounds the next terms by 3e-25
and 5e-24 respectively; smaller angles need fewer terms. The table and
the three matrix templates derive all nine cases.

Second arithmetic route: convert each moment to 1, 8 or 20 N·m and
k to 10 N·m/radian, giving the same θ. Independently use
`sin θ = 2 sin(θ/2) cos(θ/2)` and `cos θ = 1 − 2 sin²(θ/2)` for the
matrix entries; the half-angle values reproduce the table. Each matrix
also has orthonormal columns and determinant +1.

## 2. `three-head-rod.json`

These are prescribed head forces on a free rod, without distributed load
or head couples. No modulus, section diameter or support reaction is
needed to recover a section moment. Stiffness cannot change this statics
calculation. s is distance along the piecewise straight centre line,
starting at head 0. Each straight leg is interpolated linearly.

For section position r(s), define the two raw sums:

```text
P(s) = sum over heads with s_i <= s of (r_i - r(s)) cross F_i
S(s) = sum over heads with s_i >  s of (r_i - r(s)) cross F_i
```

The fixture's `momentNmm` and `prefixMomentNmm` are P(s). This is the
cut couple exerted by the prefix on the suffix. On the other cut face
the internal couple is −P(s). `suffixMomentNmm` is the raw S(s), so
P(s) = −S(s); negate S to obtain the same signed answer from that side.
Including a head exactly at the cut adds zero because its arm is zero.

The component formula used throughout is:

```text
(a, b, c) cross (u, v, w) = (b w - c v, c u - a w, a v - b u)
```

### Straight rod

The heads are at `(0,0,0)`, `(0,0,20)` and `(0,0,40)` mm, with forces
`(6,−9,4)`, `(−12,18,−8)` and `(6,−9,4)` N. Both spans are 20 mm,
the full length is 40 mm, and r(s) = (0,0,s).

Force balance, one component at a time:

```text
sum F = (6 - 12 + 6, -9 + 18 - 9, 4 - 8 + 4) = (0,0,0) N
```

Moments about the origin are:

```text
r0 cross F0 = (0,0,0)
r1 cross F1 = (-20*18, 20*(-12), 0) = (-360,-240,0)
r2 cross F2 = (-40*(-9), 40*6, 0)   = (360,240,0)
sum = (0,0,0) N·mm
```

For 0 <= s <= 20, only head 0 contributes a nonzero prefix arm:

```text
P(s) = (0,0,-s) cross (6,-9,4) = (-9s,-6s,0)
```

For 20 <= s <= 40, use the suffix head 2 to simplify:

```text
S(s) = (0,0,40-s) cross (6,-9,4) = (9(40-s),6(40-s),0)
P(s) = -S(s) = (-9(40-s),-6(40-s),0)
```

At s = 20, head 1 has zero arm and either expression gives
(−180,−120,0). At s = 40 the suffix is empty and the balanced prefix
is zero. These substitutions give every station in the file:

| s (mm) | r(s) (mm) | P(s) (N·mm) | S(s) (N·mm) |
|---|---|---|---|
| 0 | (0,0,0) | (0,0,0) | (0,0,0) |
| 5 | (0,0,5) | (−45,−30,0) | (45,30,0) |
| 10 | (0,0,10) | (−90,−60,0) | (90,60,0) |
| 15 | (0,0,15) | (−135,−90,0) | (135,90,0) |
| 20 | (0,0,20) | (−180,−120,0) | (180,120,0) |
| 25 | (0,0,25) | (−135,−90,0) | (135,90,0) |
| 30 | (0,0,30) | (−90,−60,0) | (90,60,0) |
| 35 | (0,0,35) | (−45,−30,0) | (45,30,0) |
| 40 | (0,0,40) | (0,0,0) | (0,0,0) |

For example the direct two-head prefix at s = 30 is
`(0,0,−30) cross F0 + (0,0,−10) cross F1`
`= (−270,−180,0) + (180,120,0) = (−90,−60,0)`.
The suffix gives (90,60,0), independently checking the sign.

### Rod bent in the x-z plane

The heads are at `(0,0,0)`, `(12,0,16)` and `(0,0,32)` mm, with forces
`(6,0,4)`, `(−12,0,0)` and `(6,0,−4)` N. The two leg vectors are
(12,0,16) and (−12,0,16), each of length
`sqrt(12² + 16²) = sqrt(400) = 20` mm. This is a prescribed planar
contour with a bend at the middle head, not a shape solved from the loads.

Force and moment balance are explicit:

```text
sum F = (6 - 12 + 6, 0, 4 + 0 - 4) = (0,0,0) N
r0 cross F0 = (0,0,0)
r1 cross F1 = (0, 16*(-12) - 12*0, 0) = (0,-192,0)
r2 cross F2 = (0, 32*6 - 0*(-4), 0)   = (0,192,0)
sum moment = (0,0,0) N·mm
```

The first-leg unit tangent is (0.6,0,0.8), so for 0 <= s <= 20:

```text
r(s) = (0.6s,0,0.8s)
P_y(s) = (-0.8s)*6 - (-0.6s)*4 = -2.4s
```

For the second leg, put t = s − 20 (0 <= t <= 20):

```text
r(s) = (12 - 0.6t, 0, 16 + 0.8t)
S_y(s) = (16 - 0.8t)*6 - (-12 + 0.6t)*(-4) = 48 - 2.4t
P_y(s) = -48 + 2.4t = -2.4(40-s)
```

All x and z moment components vanish because both arms and forces lie
in the x-z plane. Substitution gives all expected positions and moments:

| s (mm) | r(s) (mm) | P(s) (N·mm) | S(s) (N·mm) |
|---|---|---|---|
| 0 | (0,0,0) | (0,0,0) | (0,0,0) |
| 5 | (3,0,4) | (0,−12,0) | (0,12,0) |
| 10 | (6,0,8) | (0,−24,0) | (0,24,0) |
| 15 | (9,0,12) | (0,−36,0) | (0,36,0) |
| 20 | (12,0,16) | (0,−48,0) | (0,48,0) |
| 25 | (9,0,20) | (0,−36,0) | (0,36,0) |
| 30 | (6,0,24) | (0,−24,0) | (0,24,0) |
| 35 | (3,0,28) | (0,−12,0) | (0,12,0) |
| 40 | (0,0,32) | (0,0,0) | (0,0,0) |

For a direct prefix check at s = 30, the head 0 arm is (−6,0,−24)
and the head 1 arm is (6,0,−8). Their y moments are
`−24*6 − (−6)*4 = −120` and `−8*(−12) − 6*0 = 96`, giving −24 N·mm.
The suffix arm (−6,0,8) gives `8*6 − (−6)*(−4) = 24` N·mm.

For either contour and every station, an independent balance identity is:

```text
P(s) + S(s) = sum r_i cross F_i - r(s) cross sum F_i = 0
```

Second arithmetic route: calculate each side separately with coordinates
in metres, then multiply N·m by 1000. For example the straight s = 10 mm
prefix is (−0.09,−0.06,0) N·m and the bent s = 10 mm prefix is
(0,−0.024,0) N·m. This also checks the mm-to-moment conversion at every
station. Given forces make the moment recovery determinate; the fixture
does not claim equilibrium alone determines arbitrary head forces.

## 3. `cantilever.json`

The uniform solid round rod lies along +x, is fully clamped at x = 0,
and has a +y end load F = 10 N, with no end moment or distributed load.
Diameter d = 5.5 mm, E = 110000 N/mm², G = 42000 N/mm² and κ = 0.9
are explicit inputs in both cases. E, G and κ are independent test inputs.
The two lengths are L = 28 and 200 mm.

First calculate the section properties, using
π = 3.1415926535897932384626433832795028841971693993751:

```text
d² = 30.25 mm²; d³ = 166.375 mm³; d⁴ = 915.0625 mm⁴
A = πd²/4 = π*7.5625 = 23.75829444277281136587 mm²
I = πd⁴/64 = π*14.2978515625 = 44.91802543086734648861 mm⁴
EI = 4940982.797395408113747 N·mm²
κGA = 898063.5299368122696300 N
```

The end-load closed forms in linear Timoshenko theory are:

```text
δ_b = FL³/(3EI)
δ_s = FL/(κGA)
δ = δ_b + δ_s
φ_tip = FL²/(2EI)
σ_max = M_max*(d/2)/I = (FL)*(d/2)/(πd⁴/64) = 32FL/(πd³)
```

Positive δ is along +y; positive section rotation φ_tip is about +z.
The requested bending tip slope is this section rotation. In shear-flexible
theory the centre-line tangent is `w'(L) = φ_tip + F/(κGA)`; the extra
shear angle is 0.00001113506969902630361 radians. Comparing φ_tip with
that centre-line tangent would compare different quantities.

For L = 28 mm, L² = 784 mm², L³ = 21952 mm³ and FL = 280 N·mm:

```text
δ_b = (10*21952)/(3*4940982.797395408113747)
    = 0.01480946935737277957358 mm
δ_s = (10*28)/898063.5299368122696300
    = 0.0003117819515727365010471 mm
δ   = 0.01480946935737277957358 + 0.0003117819515727365010471
    = 0.01512125130894551607463 mm
φ_tip = (10*784)/(2*4940982.797395408113747)
      = 0.0007933644298592560485848 radians
σ_max = (32*10*28)/(π*166.375)
      = 17.14233857374463962121 MPa
```

For L = 200 mm, L² = 40000 mm², L³ = 8000000 mm³ and FL = 2000 N·mm:

```text
δ_b = (10*8000000)/(3*4940982.797395408113747)
    = 5.397036937818068357720 mm
δ_s = (10*200)/898063.5299368122696300
    = 0.002227013939805260721765 mm
δ   = 5.397036937818068357720 + 0.002227013939805260721765
    = 5.399263951757873618442 mm
φ_tip = (10*40000)/(2*4940982.797395408113747)
      = 0.04047777703363551268290 radians
σ_max = (32*10*200)/(π*166.375)
      = 122.4452755267474258658 MPa
```

The relative importance of shear follows without π or F:

```text
δ_s/δ_b = 3EI/(κGA L²) = 3E d²/(16κG L²)
L = 28:  0.02105287799643666990606, or 2.105287799643667 percent
L = 200: 0.0004126364087301587301587, or 0.04126364087301587 percent
```

Thus the short case detects omission of shear clearly. The long case is
bending-dominated, although its small nonzero shear term is still checked.

An exact shear-flexible Timoshenko beam element reproduces the end-loaded
cantilever's nodal tip deflection and section rotation with one element;
2–4 elements per span reproduce the same values. This follows from the
constant shear force and linear bending moment, with no distributed load.
There is no mesh-truncation error to hide in a broad tolerance. An
interpolation that suffers shear locking does not have this property.
This statement concerns the specified linear beam equations, not a
large-deflection or solid-mechanics solution.

Second arithmetic route: use SI units, radius r = 0.00275 m,
E = 1.1e11 N/m², G = 4.2e10 N/m², and L = 0.028 or 0.2 m:

```text
A = πr² = 2.375829444277281136587e-5 m²
I = πr⁴/4 = 4.491802543086734648861e-11 m⁴
M(x) = F(L-x); V(x) = F
δ_b = integral from 0 to L of F(L-x)²/(EI) dx
δ_s = integral from 0 to L of F/(κGA) dx
φ_tip = integral from 0 to L of F(L-x)/(EI) dx
σ_max = FLr/I
```

The integrals give bending deflections
0.00001480946935737277957358 and 0.005397036937818068357720 m,
shear deflections 0.0000003117819515727365010471 and
0.000002227013939805260721765 m, and stresses
17142338.57374463962121 and 122445275.5267474258658 N/m².
Multiply deflections by 1000 and divide stresses by 1e6; the answers
above, including the two totals and section rotations, are recovered.

## 4. `two-springs-in-series.json`

The first end is fixed, the junction is unloaded and massless, and the
free end carries the stated force. Both rest extensions are zero. Force
balance makes the transmitted force F the same in both springs:

```text
x1 = F/k1; x2 = F/k2
x = x1 + x2 = F(1/k1 + 1/k2)
k_series = F/x = 1/(1/k1 + 1/k2) = k1 k2/(k1 + k2)
```

For the +x case:

```text
k1 = 120 N/mm; k2 = 300 N/mm; F = 42 N
k_series = (120*300)/(120+300) = 36000/420
         = 600/7 = 85.71428571428571428571 N/mm
x1 = 42/120 = 0.35 mm; x2 = 42/300 = 0.14 mm
x = 0.35 + 0.14 = 0.49 mm
```

For the signed +y-axis case, F is negative, so both increments are negative:

```text
k1 = 100 N/mm; k2 = 400 N/mm; F = -80 N
k_series = (100*400)/(100+400) = 40000/500 = 80 N/mm
x1 = -80/100 = -0.8 mm; x2 = -80/400 = -0.2 mm
x = -0.8 - 0.2 = -1 mm
```

For two rotational springs about the same axis, replace x by θ and F by
M. Both rest rotations are zero. The unloaded junction transmits the
same M; coaxial rotation matrices commute, so the rotations add exactly.

```text
θ1 = M/k1; θ2 = M/k2; θ = θ1 + θ2
k_series = k1 k2/(k1+k2)
```

For the positive rotation about +x:

```text
k1 = 12000 N·mm/radian; k2 = 30000 N·mm/radian; M = 4200 N·mm
k_series = 360000000/42000 = 60000/7
         = 8571.428571428571428571 N·mm/radian
θ1 = 4200/12000 = 0.35 radians; θ2 = 4200/30000 = 0.14 radians
θ = 0.35 + 0.14 = 0.49 radians
```

For the negative rotation about +z:

```text
k1 = 10000 N·mm/radian; k2 = 40000 N·mm/radian; M = -8000 N·mm
k_series = 400000000/50000 = 8000 N·mm/radian
θ1 = -8000/10000 = -0.8 radians; θ2 = -8000/40000 = -0.2 radians
θ = -0.8 - 0.2 = -1 radian
```

Second arithmetic route: impose the reported total displacement and solve
the junction equation `k1 x1 = k2(x - x1)` instead of adding compliances.
It gives `x1 = k2 x/(k1+k2)`, then x2 = x − x1. For the first case,
`300*0.49/420 = 0.35`, `120*0.35 = 300*0.14 = 42`, and
`42/0.49 = 600/7`. For the second,
`400*(-1)/500 = -0.8`, `100*(-0.8) = 400*(-0.2) = -80`, and
`(-80)/(-1) = 80`. The rotational junction equation gives the identical
angles and checks `12000*0.35 = 30000*0.14 = 4200` and
`10000*(-0.8) = 40000*(-0.2) = -8000` N·mm. Independently converting
rotational stiffnesses and moments to N·m also leaves every angle unchanged.

## 5. `goodman.json`

All stresses are scalar longitudinal stresses in MPa, tensile-positive.
Amplitude a is the nonnegative half range, m the cycle mean and u the
positive tensile ultimate. The equivalent fully reversed amplitude is:

```text
D = 1 - m/u
a_reversed = a/D, for m < u
```

Each case's denominator and amplitude are obtained as follows:

| a | m | u | D = 1 − m/u | a/D (MPa) |
|---|---|---|---|---|
| 100 | 0 | 1000 | 1 − 0/1000 = 1 | 100/1 = 100 |
| 100 | 250 | 1000 | 1 − 250/1000 = 0.75 | 100/0.75 = 400/3 = 133.3333333333333333333 |
| 150 | 500 | 1000 | 1 − 500/1000 = 0.5 | 150/0.5 = 300 |
| 40 | 600 | 800 | 1 − 600/800 = 0.25 | 40/0.25 = 160 |
| 25 | 999 | 1000 | 1 − 999/1000 = 0.001 | 25/0.001 = 25000 |
| 25 | 1000 | 1000 | 1 − 1000/1000 = 0 | no finite answer |
| 25 | 1100 | 1000 | 1 − 1100/1000 = −0.1 | outside the conversion's finite-amplitude domain |

As a positive amplitude's mean approaches ultimate from below, D tends
to zero from above and a/D grows without bound. At m = u the fixture
expects `status: "no-finite-answer"` and `equivalentAmplitudeMpa: null`.
At m > u it expects the same status and `null`: a negative algebraic
quotient is not a fully reversed amplitude. No Infinity or NaN token is
stored in JSON. Status is a fixture convention, not a shared engine error
code. The denominator remains an ordinary finite number in every case.

Second arithmetic route: use the stress gap rather than subtracting
nearly equal fractions, `a_reversed = a*u/(u-m)`. The five finite cases
give `100000/1000 = 100`, `100000/750 = 400/3`, `150000/500 = 300`,
`32000/200 = 160` and `25000/1 = 25000` MPa. As an inverse check,
`a/a_reversed + m/u = 1` gives respectively `1+0`, `0.75+0.25`,
`0.5+0.5`, `0.25+0.75` and `0.001+0.999`. A zero or negative stress
gap rejects the last two cases before division.
