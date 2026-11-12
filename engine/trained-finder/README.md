# trained-finder

A small trained network that finds the vertebrae on a real front film,
written as plain Rust with its weights compiled in. It runs the same
natively and in WebAssembly, and learns nothing at run time. Why the
engine has it, and why only real front films get it:
`docs/decisions/a-trained-finder-for-front-films.md`.

## What happens to a film, in order

1. **Intake** (`intake`). The film is read through its record, as every
   pass reads one. It first becomes 8-bit grey: a film stored in 8 bits
   goes in as stored; a deeper one goes in as it first shows on screen,
   through its opening window, scaled to 0 to 255. A film stored with
   bone dark (MONOCHROME1) was already turned bone bright when it was
   opened, and where the user flipped the film's brightness, the
   record's flip is applied here. Blanked boxes then go to black, and the
   film is turned upright by its quarter turns. The picture is then
   shrunk to 384 by 768, stretched to fit, exactly as the training code
   shrank it, and each value becomes `value / 255 - 0.5`.
2. **The first reading** (`spine_net`, built from `weights`, `layers`
   and `grid`). The main network gives three maps at a quarter of its
   input size, 96 by 192: how likely a vertebra's centre lies in each
   cell, where that vertebra's four corners lie from there, and where the
   centre sits inside the cell. The reading is cut into small steps.
3. **Peaks** (`reading`, `overlap`). Every local peak scoring 0.05 or
   more becomes a candidate outline. Peaks scoring 0.5 or more, with
   overlapping ones dropped, are kept for the framing test alone.
4. **Framing** (`framing`). If the first reading shows a spine that is
   small on the film, as on a full-length film, the film is cropped to
   the spine and read again; otherwise the first reading stands.
5. **The chain** (`chain`). From the candidates, a search picks at most
   seventeen, T1 to L5, top to bottom, scoring each pick by the
   network's score, its height on the picture, and the gap, size change
   and sideways step from the one above, against priors learned from
   training films. One missing level between two picked vertebrae may
   be filled in halfway between them; a filled-in vertebra is marked as
   such. Levels missing at the top or the bottom are left out, never
   guessed.
6. **Corners** (`corners`). For each pick, a 128 by 128 patch is cut
   from the full-size film around it, and a second, smaller network
   says where each corner should be. Each corner moves three eighths of
   the way there, and the move is capped at 0.09375 of the vertebra's
   height.
7. **Marks** (`to_marks`). The answer leaves the crate in original film
   pixels.

`watch` holds the stepper the browser drives; `run` is the same work in
one call, for native use.

## What it returns

The same `FilmMarks` the auto pass returns for a front film, so the case
store, the checks, the drawing and the saving need nothing new:

- one vertebra per pick, top to bottom, with four body corners placed by
  the engine; a filled-in vertebra has all four corners marked inferred;
- pedicles "not visible": the network does not look for them;
- a score per vertebra: the network's own score, 0 for a filled-in one,
  not calibrated;
- level names carrying the `estimated` flag;
- a low-evidence warning when framing gave up for lack of evidence.

A vertebra "needs a look" when it was filled in or scores under 0.60.
Nothing found at all is an error, not an empty answer. A film narrower or
shorter than 64 pixels is refused as too small, the hand-built pass's own
floor.

## How it is proved

1. **Known answers, stage by stage, from a made-up film**
   (`tests/known-answers/`). The training code's own pipeline was run
   on one made-up front film and every stage written down: the shrunk
   pictures, the network's maps, the peaks, the frame, the chain's
   picks and the settled corners. The port must give the shrunk pictures
   and patches byte for byte, and the rest within stated tolerances. No
   real film is used.
2. **The same bits natively and in the browser.** The forward pass adds
   its sums in one fixed order and takes its maths from `libm`
   (`docs/decisions/libm-for-matching-maths.md`), so a reading in
   WebAssembly matches the native one bit for bit.
3. **Real films, on one machine only.** The films are not in git, so CI
   cannot score them. The port is run on the 128 test films on a machine
   that holds them, and must reproduce the figures below on the 112 from
   children with no film in training; the run is quoted in the pull
   request.

## How well it does

Measured once, after everything was frozen, on the 112 test films from
children with no film among those the network was trained on: 1,904
labelled vertebrae, 17 a film. A vertebra is found when an outline
overlaps it by a half or more (shared area over combined area), matched
one to one, the best overlap first. Corner error is each found corner's
distance from the labelled one, as a share of that vertebra's height,
comparing left and right corners either way round, whichever is closer.

| On the 112 films | Trained finder | Hand-built search |
|---|---:|---:|
| vertebrae found | 1,768 of 1,904 (92.9%) | 1,067 of 1,904 (56.0%) |
| outlines on nothing | 136 | 950 |
| films with 16 or 17 found | 85 of 112 | 1 of 112 |
| films with all 17 found and in order | 44 of 112 | 0 of 112 |
| corner error, median | 0.12 | 0.26 |
| corner error, nine in ten within | 0.28 | 0.57 |
| first outline is T1 | 62 of 112 | 2 of 112 |
| last outline is L5 | 54 of 112 | 17 of 112 |

The trained finder placed seventeen outlines on every one of these
films. Seventeen outlines is not seventeen vertebrae found: the count
comes from the chain, not from the film.

## Where it fails

- **Level names are estimates.** The commonest fault is the whole stack
  named one level too high, with an extra outline above T1 and L5
  missed (about 40 of the 112 films). The names are flagged, and the
  screen says so.
- **Scores are a guide, not a measure.** On the 108 films used to judge
  the finder's later parts, outlines under 0.60 sat on a vertebra 67
  times in 98, against 1,714 in 1,738 at 0.60 and above. The network had
  trained on 97 of those films, so on new films the split is likely
  less clean.
- **Hard films.** Braces and other hardware, soft or narrow films, and
  necks and pelvises in view give outlines on nothing and missed
  vertebrae.
- **Full-length films.** On 62 full-length films whose labels could be
  placed with confidence, it found 953 of 1,054 vertebrae, and its
  corners were further off than on the cropped films (median 0.14 of a
  height). These 62 were likely the easier ones.
- **Made-up films.** On six made-up front films it found 53 of 102. The
  app sends made-up films to the hand-built search.
- **Deep films.** No real 16-bit film was available to test the window
  rule; only made-up ones.
- **One hospital.** It was trained and tested on films from one
  hospital's collection, and is not validated for clinical use.

## Limits

- **Brightness is taken as shown.** The finder reads a film's
  brightness the way it is shown and does not check it, so a film stored
  bone-dark reads badly: on the made-up film turned bone-dark it placed
  12 outlines, misplaced and misnamed, every one scoring under 0.60, with
  no warning. The hand-built pass checks which way the brightness runs;
  this one does not.
- **Another decoder's pixels.** The figures above were measured on
  pixels decoded by another JPEG library. Decoding the same 128 test
  films with the engine's own decoder changed no film's list of
  outlines, moved no corner by more than 0.18 of a pixel, and left the
  scored figures the same.
- **Tilts read small.** On 106 of the 112 films, each vertebra's tilt
  from the finder's corners against the same from the labels has a
  slope of 0.917. The largest difference in tilt between two vertebrae
  on a film reads 2.0 degrees small on average: 3.8 degrees small where
  the labels give 40 degrees or more (46 films), and 0.6 degrees large
  where they give under 20 (21 films). No correction is applied, because
  one fitted on the test films would spoil them as a test.
- **The engine's own scorer counts differently.** On the 128 test films,
  `cargo spine score trained-finder-front` finds 2,021 of 2,171 eligible
  vertebrae, against 1,211 for the hand-built search. Its rules are not
  the reference scorer's: it leaves out flagged labels, matches outlines
  to vertebrae by the best total overlap, and measures corner error
  against the vertebra of the same name. A stack named one level off is
  then compared with its neighbour, which is why its corner error reads
  larger than the reference's.
- **The opening window can change.** Films stored in fewer than 8 bits,
  and deep films, go through their opening window, and that window is
  worked out again when a saved case is reopened, so the picture the
  finder is shown, and its answer, can come out differently after a case
  is saved and reopened.
- **Large films take longer a step.** On a film of tens of megapixels,
  the first step (reading the film into the picture the network is
  shown) and each corner take tens to hundreds of milliseconds, rather
  than a few.
- **Where nothing is found.** When the first reading has fewer than
  three peaks scoring 0.25 or more (the framing's own bar for proposing
  where the spine is, so the film is left whole for lack of evidence) and
  no vertebra in the chain scores 0.25 or more, the finder finds
  nothing. This is the one place the engine departs from the reference
  pipeline, which hands back the chain's weak guesses: following it, the
  finder placed one to four outlines on blank films, every one scoring
  under 0.25, where there is nothing to find. None of the 128 test films
  is affected: every one of them was left whole because its spine
  already showed tall and long.

## Re-making the weights and the known answers

`finder-prep/` holds the two scripts that write `weights/` and
`tests/known-answers/`, and says when to run them. They need the
training folder, which is not in this repository. What the weights are
and what they were trained on: `weights/README.md`.
