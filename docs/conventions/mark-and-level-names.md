# Mark and level names

These names are exact. They appear the same in Rust, TypeScript, JSON and
the screens' code; a test (`engine/shapes/tests/mark_names_are_exact.rs`)
fails if a serialised name ever changes.

## Marks on each kind of film

| Film | Marks per vertebra |
|---|---|
| front (`front`) | `supL`, `supR`, `infL`, `infR`: the four body corners; `pedL`, `pedR`: the two pedicle centres |
| side (`side`) | `supA`, `supP`, `infA`, `infP`: the four body corners |
| bending (`bend-left`, `bend-right`) | `sup` and `inf`: the two endplate lines, each with its ends ordered patient left, then patient right |

- `sup` is the upper endplate, `inf` the lower one.
- `L` and `R` are the **patient's** left and right, never the screen's.
  Which screen side that is comes from the film's `imageRight`.
- `A` and `P` are anterior (front) and posterior (back).
- Only pedicles can be **not visible** (`{ "seen": "not-visible" }`). The
  types allow that state nowhere else.

A bending film's points have no mark names of their own. A tool that
moves one names it by line and end: `{ on: 'endplate', vertebra, line:
'sup' | 'inf', end: 'patient-left' | 'patient-right' }`.

### Quad order

Wherever a vertebra's four corners are listed together (`OutlineQuad`,
`corners()` in Rust), they go round the outline, so drawing them in order
never crosses itself: upper L (or A), upper R (or P), lower R (or P),
lower L (or A). That is `supL, supR, infR, infL` on a front film, `supA,
supP, infP, infA` on a side film, and on a bending film the upper line's
ends then the lower line's ends reversed.

### Provisional names

A front or bending film's L and R (and a side film's A and P) depend on
which way the film faces. Each film's marks record `namedFor`: the
film's `imageRight` at its `quarterTurns` at the moment the names were
given, confirmed or not, or the engine's own guess while `imageRight` is
still unknown, so until the facing is set the names are provisional. A
`set-geometry` edit treats the marks by what it changes, judged against
the film's geometry before it:

- **The quarter turns change:** the film's marks are dropped. The
  upright direction decides which endplate is upper and the order of
  the levels, so the names can no longer be trusted. The screens do not
  start the auto pass on their own: the film shows as not yet searched
  until the user runs it again.
- **The turns stay and `imageRight` changes to a facing (not back to
  unknown):** the marks are renamed for it. If `namedFor` already is
  that facing nothing changes; otherwise every left/right (or
  anterior/posterior) pair swaps: each vertebra's corners, a front
  film's pedicles (with their seen state), the two ends of each of a
  bending film's endplate lines, the sacrum top's two ends and the
  `left` and `right` hip centres. Each point keeps its position and its
  flags (source, pinned, inferred); only which name holds which point
  changes. `namedFor` becomes the new `imageRight`.
  `FilmMarks::renamed_for` in `engine/shapes/src/vertebra.rs` is the
  definition; the app's case reducer mirrors it and must reproduce
  `proving-ground/known-answers/side-swap.json`.
- **Only the scale or the confirmations change, or `imageRight` goes
  back to unknown:** the marks are untouched, even when `namedFor`
  differs from `imageRight`. Confirming the facing as it stands never
  renames anything.

Either way it is one undo step, so undo brings the marks back with the
old turns or facing. The case check `marks-named-for-other-facing`
therefore fires only while `imageRight` is unknown, or on a file written
by something other than this app.

## Landmarks

| Name | What |
|---|---|
| `c7` | the centre of the C7 body: the mean of the chain's C7 corners, worked out again whenever they move or the names change which vertebra is C7, so never moved or pinned on its own, and `null` when the film has no C7 |
| `sacL`, `sacR` (front) or `sacA`, `sacP` (side) | the two ends of the top of the sacrum |
| hip centres | femoral head centres: `left` and `right` on a front film; on a side film one or two `unpaired` heads, or one `merged` midpoint |

## Levels

Levels run top to bottom, and that order is the order of the `Level` type:

```
C1 < … < C7 < T1 < … < T12 < T13 < L1 < … < L5 < L6 < L7
```

- The usual spine has **17** vertebrae from T1 to L5 (12 thoracic, 5
  lumbar). The chain search solves C7 as body 0, so a standard chain holds
  18 bodies.
- `T13` and `L6` exist only for spines with an extra thoracic or lumbar
  vertebra; one fewer is just as possible. Counts that differ from 12 and
  5, and transitional vertebrae at either end of the lumbar spine, are
  common enough that every film's naming is checked from both ends and
  L5 is confirmed by the user. In one MR study of 147 subjects, 8.2% had
  an unusual count and 15.0% a lumbosacral transitional vertebra
  (Carrino et al., Radiology 2011, PMID 21436097).
- Each count (`Template`) has its levels, the chain the auto pass solves
  (`TEMPLATE_LEVELS`), and its names (`TEMPLATE_NAMES`): its levels run on
  upward through C6 to C1 and downward through the lumbar names after its
  last, to L7. A boundary wand split or merge shifts the names on one
  side of the edit by one step along that list, and confirming L5 shifts
  every name to fit. Shifting never invents a thoracic vertebra: T13 is a
  name only under `extra-thoracic`. A level outside the count's own
  levels is always flagged `non-standard-count`, and an edit that would
  need a name past C1 or L7 is refused.
- Within a film, levels are strictly in order and never repeat. Each
  film's chain is its own, so a front and a side film may disagree; the
  case checks report it.

## Where it lives

- `engine/shapes/src/marks.rs`, `landmarks.rs`, `levels.rs`
- `engine/shapes/src/naming.rs`: each count's levels and names
- `app/src/shapes/mark-names.ts`: the mark names as `const` lists, checked
  both ways against the generated types; the level lists are generated
  into `app/src/shapes/generated/constants.ts`.
