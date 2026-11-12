# finder-prep

The two scripts that turn the trained finder's training run into what
the engine compiles in and tests against. They are run by hand, when
the weights change or the made-up film changes; no build runs them.
Their output is committed in `engine/trained-finder/`.

## What they need

- Python 3 with torch, numpy and Pillow.
- The training folder: the run that trained the finder, holding its
  frozen checkpoints and pipeline. It is not in this repository, and
  nor are the films it was trained on. Both scripts take its path as
  their first argument and only read from it.

What the weights are and what they were trained on:
`engine/trained-finder/weights/README.md`.

## pack_weights.py

```sh
python finder-prep/pack_weights.py <training-folder> engine/trained-finder/weights
```

Reads the two frozen checkpoints, the main network's and the corner
network's, and writes:

| File | Holds |
|---|---|
| `finder-weights.bin` | every tensor as little-endian float32, end to end |
| `finder-weights.json` | each tensor's name, shape and place in the `.bin`, the score threshold, the corner blend, and the SHA-256 of every file written and of the two checkpoints |
| `chain-priors.json` | the chain's priors, byte for byte as trained |
| `framing.json` | the framing settings, byte for byte as trained |

The main network's batch norms are folded into the convolutions before
them, in float64, and rounded to float32 once: the engine then runs one
convolution where training ran a convolution and a normalisation. The
corner network uses group norm, which depends on the picture and cannot
be folded, so its tensors are written as they are. It prints the number
of tensors and bytes written.

## make_known_answers.py

```sh
spine make-up straight --seed 7 --tier t2 --out <folder>
python finder-prep/make_known_answers.py <training-folder> \
    <folder>/straight-7-t2/front.png engine/trained-finder/tests/known-answers
```

(`spine` is the engine's command line, which `cargo spine` runs.)

Runs the training run's own Python pipeline on one made-up front film
and records what every stage gave: the two pictures the network was
shown, its maps, the peaks, the frame, the chain's picks and the
settled corners. The engine's port must give the same, stage by stage.
Only a made-up film is ever used.

The made-up film is a 16-bit PNG and the network reads 8-bit pictures,
so this script keeps each value's top eight bits. That rule belongs to
these fixtures alone: it is not how the engine reads a deep film.

It writes `manifest.json`, `pipeline.json`, `chain-cases.json` and the
`.bin` arrays; `engine/trained-finder/tests/known-answers/README.md`
says what each holds and why the `*_u8.bin` files stay out of git. It
prints the frame, the number of peaks on each reading and the length
of the chain.

## When to run them

- **New weights:** run both. The known answers come from the weights,
  so they change too.
- **A change to the made-up film generator** that changes the film's
  bytes: run `make_known_answers.py`. The crate's tests check the film's
  SHA-256 first and fail until you do.
