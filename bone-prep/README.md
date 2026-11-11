# bone-prep

The one script that prepares the vertebra shapes the 3D view draws. It is
run once, by hand, when the shapes change; the app never runs it. Its
output is committed in `app/public/bones/`.

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
   millimetres.

No shape is altered: nothing is smoothed, simplified or rescaled. The
same archive always gives the same bytes.

## What it writes

| File | Holds |
|---|---|
| `app/public/bones/vertebrae.json` | the credit, and for each bone its level, its body's sizes, where it stood in the source, and where its numbers start in the other file |
| `app/public/bones/vertebrae.bin` | every bone's points (float32 x, y, z), then every bone's triangles (uint16 corners), little-endian |

`app/src/spine-3d/bones/read-bones.ts` reads the pair, and its test
checks the committed files: eighteen bones in order, every surface
closed and facing outwards, every body centred in its frame.

## Placing a bone

A fitted vertebra has a centre, a rotation and a body height, width and
depth. Its bone is drawn at

```
centre + rotation × (scale × point)
```

where `scale` stretches x by fitted depth over the bone's depth, y by
fitted width over the bone's width, and z by fitted height over the
bone's height. The shape is one adult's bone stretched to size: a
generic shape, never the patient's own.
