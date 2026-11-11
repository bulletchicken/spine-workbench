# sample-case

The one fixed story every screen can be opened in: the design mock's
made-up spine, its four films, and the case at each moment of the
design's flow. The stand-in engine answers from it in development, and
gallery pages, scenes and tests read it. It never ships, and nothing in
it comes from a patient: every film is painted from the numbers in
`anatomy.ts`.

| File | Holds |
|---|---|
| `anatomy.ts` | the mock's spine, number for number: the bones of each film, the centre line, the marker and the ball |
| `film-painter.ts`, `painting/` | the mock's painter, ported call for call onto plain arrays |
| `films.ts` | each film as the file a person drops, what reading it finds, its half-float pixels and its thumbnail |
| `files/` | the PNG, DICOM and zip writers, and their checksums |
| `guesses.ts` | what sensing each film's geometry answers |
| `marks.ts` | each film's marks as the automatic pass leaves them |
| `stages.ts` | `caseAt(stage)` and `findingsAt(stage)` |
| `refused-files.ts` | a file that is not a film, and a case that will not open |
| `keyframes-only.ts` | a short pass made from the marks, which the watch-frame rule keeps whole |
| `answer-key.ts` | the truth the films were painted from, as a made-up spine's answer key |
| `story.ts` | all of it as the one `StandInStory` the stand-in finds by path |
| `story-shape.ts` | the `StandInStory` type |
| `films.gallery.tsx` | the four films on plain canvases, to compare with the mock's 04-up |

## The story, stage by stage

| Stage | The case |
|---|---|
| `four-films-up` | four films hung, nothing known about them |
| `films-all-set` | the front film's left and right answered (the patient's left on the image right, as its lead L says); bending right measured across its 25 mm ball; bending left turned upright; the other sizes read from the files |
| `pass-result` | "Find the spine" pressed, which confirms upright, facing and size for the facts shown, and the pass landed |
| `l5-set` | L5 confirmed on every film |
| `after-split` | T11 and T12, taken as one, split on Front and Side |
| `t3-fixed` | the region wand added the bone T3's outline missed, on Front |
| `all-clear` | T1 confirmed with "Looks right" on Front and Side |

Front and Side hold 18 bodies after the pass: the C7 body the engine's
chain always has, and the design's 17, among them T11 and T12 as one
outline twice as tall as its neighbours (low, `odd-height`), the lowest
outline on the top of the sacrum (medium, `level-uncertain`), T3 short
of its bone on Front (low, `rib-overlap`) and T1 with its left pedicle
not visible on Front (medium, `pedicle-hidden`). Counted from the first
rib, the outline on the sacrum is L5; counted up from the sacrum, the
one above it is.

Ids start at 1 for the front film's C7, run on film after film to 72,
and the split's lower pieces take 73 (Front) and 74 (Side).

## Where the sample case differs from the mock

The frozen rules and the repository's conventions win where they and
the mock disagree. These are the differences a builder comparing a
screen with a screenshot will meet.

- **The pixel size is 0.45 mm.** The mock painted at 0.6 mm a pixel and
  printed 0.144. At 0.45 the bodies come out at an adult's size, and the
  25 mm ball spans 55.6 pixels, enough for a two-click ruler; so the
  ball is a little larger than the mock's, the size chips read 0.45, and
  bending right measures 0.45 mm a pixel, not 0.151.
- **Three films are DICOM files.** A size "read from the file" needs the
  file's kept tags, which only a DICOM file has, so front, side and
  bending left are DICOM; bending right, a PNG, has no size.
- **The C7 body is part of the chain**: the chain search solves C7 as
  body 0, so a standard chain holds 18 bodies
  (`docs/conventions/mark-and-level-names.md`). The app draws it in the
  not-counted style.
- **Confirming L5 renames to the letter of `confirm-l5`.** Every name
  moves one down, so at `l5-set` the C7 body is named T1, the real T1 is
  T2, `landmarks.c7` is empty because no vertebra is C7, and every level
  from T1 to L5 has a vertebra; the missing one is known only by the
  tall outline's `odd-height`. The mock, which has no C7 body, shows the
  real T1 as T2 at that point too.
- **The outline on the sacrum is L6 once L5 is confirmed**, flagged
  `non-standard-count`; the design calls it S1, not counted.
- **A split shifts the names above it.** The frozen rules shift the names
  on one side of the edit; here it is the side away from the confirmed
  L5, so L5 keeps its name and its confirmation and C7 comes back. This
  is a choice the frozen text leaves open, and the boundary wand on main
  makes the other one: `engine/toolbox/src/boundary_wand/renumber.rs`
  says every name below the edit shifts. Under that rule the split would
  move the confirmed L5's name, clear `l5ConfirmedOn` and raise a naming
  conflict, so `after-split` here and the wand's renaming have to be
  brought to one rule before either is relied on.
- **The bending films' chains are right**, C7 to L5. The mock draws the
  same 17 outlines on them, the tall one included, but no edit can change
  a bending film's count, so a merged outline there could never be fixed.
- **Confidence is per film.** T3 is low on Front only; the side film shows
  it clearly. T1 is medium on both.
- **The side film's hips are two heads**, painted 8.5 mm apart, so they
  are two unpaired hip centres; the mock draws one circle for both.
- **The pass is its keyframes only** until the find frames are recorded
  from the films.
- **Pedicles the pass cannot find are placed where they usually sit and
  marked inferred**: on the C7 body, on the outline on the sacrum, and on
  the split's lower piece.
- **The lead letter is drawn as a shape**, not set in a typeface, so its
  edges differ from the mock's by a pixel or two.
- **The answer key's recipe does not remake the case.** The films are
  painted from `anatomy.ts`, not drawn from a recipe; the key records
  the anatomy's seed under `random` at tier `t2` only because a made-up
  spine's key must name one. Making up a spine from that recipe gives a
  different spine.
- **The answer key's twist follows the painted pedicles**, which sit
  toward the patient's right at the thoracic apex: read as a turn, the
  front of the body faces the patient's left there.
