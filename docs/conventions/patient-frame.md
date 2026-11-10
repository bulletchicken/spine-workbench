# The patient frame

Three-dimensional positions are in the **patient frame**:

- units are **millimetres**;
- **x** points forward (anterior);
- **y** points to the **patient's left**;
- **z** points up (toward the head);
- the **origin** is the centre of the underside of L5 (its lower endplate).

The axes are right-handed: x × y = z.

## Films in the patient frame

A film is a projection of the patient frame. Which patient direction lies
along the film's horizontal is the film's `imageRight` (after its quarter
turns):

| `imageRight` | screen right is | across = |
|---|---|---|
| `patient-left` | +y | y |
| `patient-right` | −y | −y |
| `anterior` | +x | x |
| `posterior` | −x | −x |

Front and bending films use `patient-left` or `patient-right`; side films
use `anterior` or `posterior`.

For the simple parallel camera, in **upright pixels** (the film shown
after its quarter turns, before any mirroring), with (u0, v0) the upright
pixel where the patient origin lands:

```
u = u0 + across / su
v = v0 − z / sv
```

v grows downward on the film while z grows upward, hence the minus sign.
`su` and `sv` are the upright pixel's width and height in mm. The stored
spacings belong to the film as stored: `mmPerPx` along a film row,
`mmPerPxVertical` (or `mmPerPx`) down a film column. After an even number
of quarter turns `su = mmPerPx` and `sv` is the vertical spacing; after
an odd number they swap. The point in original film pixels is the
upright one with the turns undone: the inverse of `uprightMatrix` in
`app/src/film-viewer/film-space.ts`, mirroring off.
Point-source and slot-scanner cameras (Wave 2) sit beside the parallel one
in `engine/forward-model/src/cameras/` and keep the same meaning of
`mmPerPx` on the plane through the origin.

## Vertebrae in the patient frame

Vertebrae are stacked upward from L5. Each one's own axes follow the same
pattern as the patient's: its x forward, its y to its left, its z up along
the line between its endplate centres. Its rotation maps those axes onto
the patient's; see [signs and wedges](signs-and-wedges.md).

## Where it lives

- `engine/forward-model/src/patient_frame.rs`
- `engine/forward-model/src/cameras/parallel.rs`
