# A trained finder for real front films

**Decided:** a real front film is read by a small trained network, run
as hand-written Rust inside the engine (`engine/trained-finder`) with
its weights compiled in. Made-up films, and every side and bending
film, keep the hand-built search. The app says which finder reads the
front film, from where the films came from; it is never guessed from
the pixels.

## Why

- **On real films the network finds far more, and places far less on
  nothing.** Measured once, on the 112 test films from children with no
  film among those the network was trained on (1,904 labelled
  vertebrae):

  | | Trained finder | Hand-built search |
  |---|---:|---:|
  | vertebrae found | 1,768 of 1,904 (92.9%) | 1,067 of 1,904 (56.0%) |
  | outlines on nothing | 136 | 950 |

  "Found" means an outline overlaps a labelled vertebra by a half or
  more (shared area over combined area), matched one to one, the best
  overlap first. "On nothing" is an outline matched to no vertebra.
- **On made-up films it is the other way round.** On six made-up front
  films the network found 53 of 102 vertebrae; the hand-built search
  finds about three in four. The network never saw a made-up film. So
  the choice follows where the film came from, not which finder is
  "better".
- **The network was trained on front films only**, so side and bending
  films stay with the hand-built search.
- **Hand-written Rust, not a runtime library.**
  - No new dependency: nothing new is downloaded, audited or pinned.
  - The network is small (3×3 convolutions, batch norm, ReLU and
    bilinear upsampling, about 2.7 million weights in all), and a
    hand-written forward pass keeps the engine's rules: no threads, no
    clocks, the same answer every time, and work cut into small steps so
    a worker never falls silent
    (`stepped-passes-for-the-watchable-sweep.md`).
  - The same code is scored natively and runs in the browser, with the
    same bits.
  - One reading takes about half a second in WebAssembly, with no
    `unsafe` code, measured on a laptop with a network of the same
    shape.
- **Weights compiled in, not fetched.** The app keeps exactly one fetch,
  its own engine (`one-sanctioned-wasm-loader.md`), so the content
  policy, the lint rule and the browser tests stay as they are.
- **float32 weights, not float16.** The port must reproduce the
  reference figures closely enough to give the same found and
  on-nothing counts. Measured on a made-up film, float16 weights moved
  the network's outputs by more than every tolerance the port is tested
  at, enough to flip its score thresholds and the chain's choices. A
  float16 version would be a different model to be judged again.

## Consequences

- The engine file grows by about 10.9 MB (10,872,220 bytes of weights,
  plus two small settings files), loaded from the user's own disk.
- **Level names are estimates.** On the 112 test films T1 was the first
  outline on 62 films, and the commonest fault is the whole stack named
  one level too high (about 40 films). The names carry the `estimated`
  flag and the screen says so in plain words.
- **"Needs a look".** A vertebra the chain filled in (the network did
  not see it) and any outline scoring under 0.60 are marked for a
  person to check. On the 108 films used to judge the finder's later
  parts, outlines under 0.60 sat on a vertebra 67 times in 98, against
  1,714 in 1,738 at 0.60 and above. The network had trained on 97 of
  those 108 films, so the split is likely kinder than it will be on new
  films. The score is the network's own, not calibrated.
- **Deep films.** A 16-bit film is turned to 8 bits through its opening
  window, as it first shows on screen, because the network was taught
  on 8-bit pictures. No real deep film was available to test this; the
  only check is on made-up 16-bit films.
- **Tilts read small.** On 106 of the 112 test films, the finder's
  vertebra tilts follow the labels' with a slope of 0.917, the largest
  difference in tilt on a film reading 2.0 degrees small on average, and
  no correction is applied, because one fitted on the test films would
  spoil them as a test.
- **Nothing found, where the reference finds weak guesses.** When the
  first reading has too little evidence to frame the film and no
  vertebra in the chain scores as high as the framing's own bar (0.25),
  the engine finds nothing where the reference pipeline hands back the
  chain's weak guesses, so that a blank or featureless film gets no
  outlines on nothing; this is the one place the engine departs from
  the reference.
- **Full-length films get a second reading** on a crop around the spine.
  On 62 full-length films whose labels could be placed with confidence,
  the finder found 953 of 1,054 vertebrae; the network reading the whole
  film once, without the crop or the later steps, found 726. These 62
  were the films whose labels could be placed, likely the easier ones,
  and the crop rule was revised after their pictures had been looked at.
- **One hospital's films.** The network was trained and tested on films
  from one hospital's collection (SpineWeb Dataset 16, as released for
  the AASCE 2019 challenge). How it does on other machines, hospitals
  or patients is not known.
- **It is not validated for clinical use.** The finder places outlines
  for a person to check.
- The films' source asks that any publication resulting from their use
  cite Wu, Bailey, Rasoulinejad and Li, MICCAI 2017,
  DOI 10.1007/978-3-319-66182-7_15. The credit is in `NOTICE` and in
  `engine/trained-finder/weights/README.md`.
- Where the numbers came from, how the weights are re-made and how the
  port is proved: `engine/trained-finder/README.md`.
