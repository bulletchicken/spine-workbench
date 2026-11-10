# Case file, version 1

A case is saved as one file, `spine-case-YYYY-MM-DD.spinecase`: a zip
holding the films and one documented data file. The engine is the only
reader and writer of case files, and it reads them strictly. Frozen:
version 1 changes only through a contract-change pull request; a later
version adds an upgrade from this one, and every version 1 file must
still read.

The JSON Schema, [`case-file-v1.schema.json`](case-file-v1.schema.json),
is generated from the Rust shapes by `cargo chores shapes`. The Rust in
`engine/shapes/src/case_file_v1.rs` and the strict reader in
`engine/shapes/src/reading.rs` are the authority where this page and they
differ.

## Inside the zip

Exactly these entries, nothing else:

| Entry | Holds |
|---|---|
| `case.json` | the record below, written by the engine in its one canonical form |
| `films/<kind>.png` | one per film present (`front`, `side`, `bend-left`, `bend-right`): a lossless greyscale PNG at the film's own bit depth, re-encoded from the decoded pixels |
| `README.txt` | a short note for a person who unzips the file, pointing at this format |

Original DICOM, JPEG or PNG bytes are never stored, so no tag, EXIF field
or file name can travel inside a case. Blanked boxes are zero in the
stored PNG: every pixel a box reaches into, not one it only meets along
an edge ([pixel space](../conventions/pixel-space.md)). The PNG holds the
values `values` describes, the user's brightness flip included.

## What is never stored

No names, identifiers, dates, institutions, free-text notes or original
file names: the types have no field for any of them, so they cannot be
saved. Values that can be worked out again from the stored PNG and the
kept tags (scale candidates, marker findings, collimation) are not saved
either. Only the app's version is recorded, in `savedWith.app`.

## The record

```
CaseFileV1
├── format: "spine-case"
├── version: 1
├── savedWith: { app }                   the app version only
├── films: Film[]                        only the films present, each kind at most once
│   ├── kind                             front | side | bend-left | bend-right
│   ├── image: { path, width, height, bitsStored }
│   ├── source                           png | jpeg | dicom
│   ├── keptTags: KeptTags | null        the DICOM keep-list; null for PNG and JPEG
│   ├── values: { offset, inverted, invertedByUser, modality }
│   ├── flags: IntakeFlag[]              what reading noticed that the PNG no longer shows
│   ├── blanked: PxRect[]                boxes the user blanked out, each inside the film
│   └── geometry
│       ├── mmPerPx, mmPerPxVertical     null until a scale is known
│       ├── scale: ScaleSource | null    file { rule } | ruler { rulers } | known-object { centre, radiusPx, diameterMm }
│       ├── quarterTurns: 0..3           clockwise turns that show it upright (view only)
│       ├── imageRight                   patient-left | patient-right | anterior | posterior | null
│       └── confirmed: { upright, leftRight, facing, scale, polarity, privacy }
├── marks: FilmMarks[]                   one per film that has marks
│   ├── film
│   ├── vertebrae: VertebraOnFilm[]      top to bottom
│   │   ├── id                           unique in the case, never reused
│   │   ├── level                        C1 … C7, T1 … T13, L1 … L7 (see below)
│   │   ├── marks                        front { points } | side { points } | bending { lines }
│   │   ├── confidence: { score, band, reasons, calibrated, endplateSpreadPx, margin }
│   │   └── outline: Px[] | null         the region wand's outline
│   ├── landmarks: { c7, sacrumTop, hips }   c7 is derived from the C7 vertebra's corners
│   ├── naming: LevelNaming              this film's count and alternatives, both end counts, the confirmed L5's id
│   └── namedFor                         the facing the L/R or A/P names were given for
└── nextVertebraId                       above every id in the case
```

Levels run C7 to L5 on a standard spine; a film's `naming.template` may
add T13 or L6 or drop T12 or L5. A split or merge with the boundary wand,
or confirming L5, shifts names along the count's list of names, so a
chain can also hold C1 to C6 above C7, or L6 and L7 below its last
lumbar; such a film is always flagged `non-standard-count` until the user
settles it. See [mark and level names](../conventions/mark-and-level-names.md).

Every point is in original film pixels
([pixel space](../conventions/pixel-space.md)). Mark and level names are
in [mark and level names](../conventions/mark-and-level-names.md). A mark
is `{ at, source, pinned, inferred }`: `source` says who last placed it
(`auto`, `region-wand`, `boundary-wand`, `pin` or `hand`), `pinned` marks
stay where the user put them, and `inferred` marks were placed without
image evidence at that spot.

A stored pixel value `p` relates to the original file's value `s` by
`s = offset + (inverted ? 2^bits − 1 − p : p)`, and `modality` maps `s` on
to modality values, so windows from the original file still apply.

## Reading strictly

`case.json` is read in four steps, each with its own error code. Every
error names the offending value's JSON path in `detail`, for example
`marks[0].vertebrae[1].level`.

1. **It must be JSON**, with no key written twice in one object (the
   path names the repeated key). Anything else, `NaN` and `Infinity`
   included, is `case-invalid-value`.
2. **Format and version.** `format` must be `"spine-case"`
   (`case-wrong-format`). `version` must be 1; a higher version is
   `case-newer-version` ("made by a newer version").
3. **Exact shape.** A field the format does not have is
   `case-unknown-field`. A missing field or a wrong type is
   `case-invalid-value`. Every field is written out, `null` included, so
   each value has exactly one spelling.
4. **Sensible values** (`case-invalid-value`), among them:
   - each film kind at most once, its image path its own
     (`films/front.png` for the front film), width and height 1 to 16384,
     1 to 16 bits;
   - marks only for films the case has, at most one set per film;
   - levels strictly top to bottom on each film, never repeated, each a
     name the film's count (`naming.template`) can give; a level outside
     the count itself needs the `non-standard-count` flag;
   - `naming.fromTop` and `naming.fromBottom`, when present, one name per
     vertebra, in order; `naming.l5ConfirmedOn`, when set, the id of that
     film's L5;
   - `landmarks.c7` null exactly when the film has no C7 vertebra, and
     otherwise the mean of that vertebra's four corners (to a thousandth
     of a pixel), never pinned, its source `auto`, inferred exactly when
     one of the corners is;
   - blanked boxes inside the film's outer edges;
   - every point finite and on or near its film: at most one film width
     or height beyond an edge, since vertebrae inferred past a cropped
     film may lie outside it;
   - scales between 0.01 and 2 mm per pixel;
   - vertebra ids unique, and `nextVertebraId` above all of them;
   - `savedWith.app` a plain version string of at most 32 characters.

The zip is held to its own limits when it is opened: only the entries
listed above, no path tricks, entry sizes and compression ratios within
bounds (`case-too-large`), every film the record lists present
(`case-missing-film`), and each PNG exactly the size its `image` says
(`case-film-size-mismatch`). Anything that is not a zip is
`case-not-a-zip`; an entry not on the list is `case-unknown-entry`.

A case built in the app is checked the same way before anything is
written: the app hands the engine a `CaseFileV1` object, and the engine
refuses it if it would not read back.

## Canonical form

Fields in declaration order, two-space indents, one newline at the end.
A file the engine wrote reads back and writes out byte for byte the same.

## Example

The smallest valid case: one front film, nothing marked yet
([`minimal.json`](../../proving-ground/known-answers/case-files/valid/minimal.json)).

```json
{
  "format": "spine-case",
  "version": 1,
  "savedWith": {
    "app": "0.1.0"
  },
  "films": [
    {
      "kind": "front",
      "image": {
        "path": "films/front.png",
        "width": 1200,
        "height": 3000,
        "bitsStored": 8
      },
      "source": "png",
      "keptTags": null,
      "values": {
        "offset": 0,
        "inverted": false,
        "invertedByUser": false,
        "modality": {
          "kind": "linear",
          "slope": 1.0,
          "intercept": 0.0
        }
      },
      "flags": [],
      "blanked": [],
      "geometry": {
        "mmPerPx": null,
        "mmPerPxVertical": null,
        "scale": null,
        "quarterTurns": 0,
        "imageRight": null,
        "confirmed": {
          "upright": false,
          "leftRight": false,
          "facing": false,
          "scale": false,
          "polarity": false,
          "privacy": false
        }
      }
    }
  ],
  "marks": [],
  "nextVertebraId": 1
}
```

A full four-film case, with DICOM tags, a turned bending film, marks,
landmarks, naming and a pinned pedicle, is
[`four-films.json`](../../proving-ground/known-answers/case-files/valid/four-films.json).
One vertebra from its front film looks like this:

```json
{
  "id": 1,
  "level": "L4",
  "marks": {
    "kind": "front",
    "points": {
      "supL": { "at": [860.5, 2400.0], "source": "auto", "pinned": false, "inferred": false },
      "supR": { "at": [700.0, 2392.25], "source": "auto", "pinned": false, "inferred": false },
      "infL": { "at": [866.0, 2560.0], "source": "auto", "pinned": false, "inferred": false },
      "infR": { "at": [704.5, 2555.5], "source": "auto", "pinned": false, "inferred": false },
      "pedL": { "seen": "visible", "at": [830.0, 2440.0], "source": "auto", "pinned": false, "inferred": false, "ringScore": 0.75 },
      "pedR": { "seen": "visible", "at": [735.0, 2437.5], "source": "pin", "pinned": true, "inferred": false, "ringScore": null }
    }
  },
  "confidence": {
    "score": 0.9375,
    "band": "high",
    "reasons": [],
    "calibrated": false,
    "endplateSpreadPx": { "sup": 1.5, "inf": 2.0 },
    "margin": 4.25
  },
  "outline": null
}
```

(The engine writes one value per line; it is folded here to fit.) The
files in `proving-ground/known-answers/case-files/` are read natively by
Rust, through the real engine in Node, and against `verdicts.json`, which
records what each must give.
