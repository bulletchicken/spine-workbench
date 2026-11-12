# bone-prep

The one script that prepares the vertebra shapes the 3D view draws. It is
run once, by hand, when the shapes change; the app never runs it. Its
output is committed in `app/src/spine-3d/bones/`, beside the code that
reads it.

The typical spine the fit falls back on, built from these shapes and
published cohorts by `typical_spine.py`, is described in
`typical-spine.md`.

## Where the shapes come from

BodyParts3D, release 4.0, from the Life Science Database Archive of the
Database Center for Life Science in Japan:
https://dbarchive.biosciencedbc.jp/en/bodyparts3d/

| File | Size | sha256 |
|---|---|---|
| `partof_BP3D_4.0_obj_99.zip` | 64,888,505 bytes | `9fbc713fffeee924a5a657d9813d84d7eb957bded63adb854931dd5e3eb61c97` |

The archive is not in this repository. Fetch it from the `LATEST` folder
of the archive's download page; the script refuses any other file.

It holds one adult man's anatomy, built from whole-body MRI and finished
by medical illustrators, as 1,258 separate surfaces. We use eighteen: the
vertebrae from C7 to L5, each its own closed surface of 1,700 to 3,900
triangles. Which file is which vertebra comes from the archive's own
table, `partof_element_parts.txt`; the ids are listed at the top of the
script.

## Licence

Creative Commons Attribution 4.0 International. The archive's licence
page (https://dbarchive.biosciencedbc.jp/en/bodyparts3d/lic.html, last
updated 27 February 2025, read on 10 November 2026) says so, and asks
for this credit, which is in `NOTICE` and in `vertebrae.json`:

> BodyParts3D, © The Database Center for Life Science licensed under CC
> Attribution 4.0 International

The surface files inside the archive were written in 2013. Their comment
headers still name the licence of that time, Creative Commons
Attribution-Share Alike 2.1 Japan, and point to the same licence page.
We go by the page the files point to.

## What the script does

```
python3 bone-prep/prepare_bones.py path/to/partof_BP3D_4.0_obj_99.zip
```

It needs Python 3.10 or later and nothing else. For each vertebra it:

1. joins the points the archive repeats, which makes the surface whole;
2. finds the vertebral body from two cuts through the bone, one down the
   middle and one across, and measures its height, width and depth from
   rim to rim;
3. moves the bone into a frame of its own: origin at the middle of the
   body, x forward, y to the patient's left, z up the body, in
   millimetres;
4. measures the body's two endplates as the vertebra model describes
   them (see "The body's endplates" below).

No shape is altered: nothing is smoothed, simplified or rescaled. The
same archive always gives the same bytes.

## The body's endplates

A bone can only be made to sit on a fitted vertebra's outline if we know
where its own endplates are. The box height above is not enough: it is
the height of the taller wall, and every bone carries its source's own
wedge (L5 is about ten degrees taller in front than behind). So for each
bone the script also writes a `body`, in the bone's own frame:

| Field | Holds |
|---|---|
| `top.centre`, `bottom.centre` | the middle of each endplate's rim, mm |
| `top.halfWidth`, `top.halfDepth` and the same for `bottom` | half the rim's span across and front to back, mm |
| `wedgeFront` | the angle between the two endplates seen from the front, radians: positive when the patient's left side is taller |
| `wedgeSide` | the same seen from the side: positive when the front is shorter |

These are the vertebra model's own numbers, signed as
`docs/conventions/signs-and-wedges.md`. How they are found:

1. The bone is cut by eighteen upright planes through its z axis. In
   each of the thirty-six directions this gives, the rim is the highest
   (for the bottom endplate, lowest) point of the body's outline over
   the outer half of its footprint, so the cupped middle of the endplate
   is left out. A cut follows the surface exactly, so the long thin
   triangles on the body's walls do not matter.
2. A plane is fitted through those rim points. A point well off it, such
   as a lip or the root of the arch, is dropped and the plane fitted
   again. On every bone the points kept lie at most 0.54 mm from their
   plane, root mean square, and none more than 1 mm.
3. The bone is cut again, 2 mm inside the body and parallel to that
   plane, and the cut is measured across and front to back through its
   middle. The rim's centre is the middle of both spans, on the plane;
   the half-sizes are half of each span.
4. The wedges are the angles between the two planes.

The script prints these numbers and their fit for every bone as it runs.

## What it writes

| File | Holds |
|---|---|
| `app/src/spine-3d/bones/vertebrae.json` | the credit, and for each bone its level, its body's sizes and endplates, where it stood in the source, and where its numbers start in the other file |
| `app/src/spine-3d/bones/vertebrae.bin` | every bone's points (float32 x, y, z), then every bone's triangles (uint16 corners), little-endian |

`app/src/spine-3d/bones/read-bones.ts` reads the pair, and its test
checks the committed files: eighteen bones in order, every surface
closed and facing outwards, every body centred in its frame, every
body's endplates inside the bone and about its height apart.

## Placing a bone

A fitted vertebra has a centre, a rotation, a height between its
endplate centres, two wedges and four endplate half-sizes. First its
bone's body is shaped to those endplates: the same centres, half-sizes
and wedges, with the bone's own wedges taken out. Between the two
plates the changes blend smoothly, so the walls stay smooth.

The arch behind the body keeps its proportions. It is scaled evenly by
the ratio of widths, rather than stretched with the body's depth or
height, and joined smoothly to the shaped body.

Every shaped point is now in the fitted vertebra's own frame. The last
step stands it in the patient:

```
centre + rotation × point
```

`app/src/spine-3d/placing/shape-bone.ts` shapes the bone and says how.
[The view](../app/src/spine-3d/README.md) describes the placing and its
checks against the fitted endplates. The shape is one adult's bone
shaped to the fit: a generic bone, never the patient's own.
