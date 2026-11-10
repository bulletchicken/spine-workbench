# Signs and wedges

One vertebra is described by **13 numbers**, all in the
[patient frame](patient-frame.md) (millimetres and radians):

| Numbers | Name | Meaning |
|---|---|---|
| 3 | centre `c` | the midpoint between the two endplate centres |
| 3 | rotation vector `r` | the rotation taking the vertebra's own axes to the patient's (axis times angle) |
| 1 | height `h` | the distance between the two endplate centres |
| 1 | front-view wedge `ωf` | **positive when the patient's left side is taller** |
| 1 | side-view wedge `ωs` | **positive when the front is shorter** |
| 2 | top half-width `wt`, top half-depth `dt` | the upper endplate's half-sizes |
| 2 | bottom half-width `wb`, bottom half-depth `db` | the lower endplate's half-sizes |

## Endplates tilt by half the wedge

Each endplate takes half of each wedge, the top one way and the bottom the
other. In the vertebra's own frame, with `ẑ` its up axis:

```
top normal     n_t = rotX(+ωf/2) · rotY(+ωs/2) · ẑ
bottom normal  n_b = rotX(−ωf/2) · rotY(−ωs/2) · ẑ
```

Both normals point up. An endplate's in-plane axes are

```
ê_w = normalise(ŷ − (ŷ · n) n)     across, toward the vertebra's left
ê_d = n × ê_w                      front to back
```

and the endplate is an ellipse with half-axes `w` along `ê_w` and `d` along
`ê_d`.

## Rotation signs

A rotation about one patient axis, by the right-hand rule:

- `rotX(+θ)` tips the column toward the patient's right (−y) going up, and
  lowers the right side of a horizontal plane;
- `rotY(+φ)` tips the column forward going up, and lowers the front;
- `rotZ(+ψ)` turns the front of the body toward the patient's left.

Made-up spines compose a vertebra's rotation as `R = rotX(tilt) ·
rotY(lean) · rotZ(twist)`.

## What a corner is

A tilted, turned endplate projects onto a film as an ellipse. The corner
of the vertebral body on that film is the end of that ellipse along the
endplate's long direction in the view. With `A` and `B` the projected
half-axes (`A = P(w · ê_w)`, `B = P(d · ê_d)` under the projection `P`) and
`d̂` the projected long direction, there is a closed form:

```
t* = atan2(d̂ · B, d̂ · A)
corners = c′ ± (A cos t* + B sin t*)
```

where `c′` is the projected endplate centre. A test checks it against a
brute-force walk around the rim.

## Pedicles

Until they are measured from a bone set, pedicle centres sit at fixed
fractions of the body, in the vertebra's own frame, `+` to its left:

```
(−1.25 · d̄,  ±0.78 · w̄,  +0.18 · h)      d̄ = (dt + db) / 2,  w̄ = (wt + wb) / 2
```

These are starting guesses. A pedicle placed this way is always marked
`inferred`.

## Curves in made-up spines

Tilt, twist and wedge along a made-up spine follow one bump shape:

```
b(u) = u · exp(−u² / 2) · √e        so b(±1) = ±1 and b′(0) = √e ≈ 1.6487
```

with `u` the distance from the curve's apex in units of its half-span.

## Where it lives

- `engine/forward-model/src/vertebra_params.rs`, `rotation.rs`,
  `endplates.rs`, `corners.rs`, `pedicles.rs`
- `engine/made-up-spines/src/curves/bump_curves.rs`
