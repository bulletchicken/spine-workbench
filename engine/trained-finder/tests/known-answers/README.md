# Known answers

What the training code's own pipeline gave, stage by stage, on one
made-up front film. The crate's tests run the port on the same film and
compare each stage. Only a made-up film is ever used here: no real
film, and nothing worked out from one, goes into the repository.

The film is the front film of
`spine make-up straight --seed 7 --tier t2`. It is not kept either: the
tests make it again with `made-up-spines` and check its SHA-256 against
the one in `manifest.json` before comparing anything.

## The files

| File | Holds |
|---|---|
| `manifest.json` | the film's SHA-256 and how it was turned to 8 bits; each saved array's type, shape and SHA-256; and for every block of the network, on each reading, its sums, its range and a small patch of numbers |
| `pipeline.json` | each reading's peaks and outlines, the frame, the chain's picks in crop and in film fractions, the corner network's patches and moves, the settled corners in film pixels, and the whole pipeline's final answer |
| `chain-cases.json` | the chain alone: no peaks at all, and the seventeen picked peaks with the 5th, 9th or 13th taken away, each with what the chain gave |
| `pass1_heat_logits_f32.bin`, `pass2_heat_logits_f32.bin` | the network's centre map, before the sigmoid, on the first reading and on the crop, 96 by 192, little-endian float32 |
| `pass1_input_u8.bin`, `pass2_input_u8.bin`, `refiner_patches_u8.bin` | the two shrunk pictures the network was shown and the seventeen corner patches, as bytes; not kept in git (below) |

The made-up film is a 16-bit PNG and the network reads 8-bit pictures,
so these answers keep each value's top eight bits. That rule belongs to
these fixtures alone: it is not how the engine reads a deep film.

## Why the `*_u8.bin` files are not kept

The shrunk pictures and the patches must match byte for byte, so their
SHA-256 in `manifest.json` is all a test needs. The script still writes
the bytes, for whoever is chasing a mismatch; `.gitignore` keeps them
out of git.

## Making them

From the repository root, with the training folder at hand (it is not
in this repository; see `finder-prep/README.md`). `spine` is the
engine's command line, which `cargo spine` runs:

```sh
spine make-up straight --seed 7 --tier t2 --out <folder>
python finder-prep/make_known_answers.py <training-folder> \
    <folder>/straight-7-t2/front.png engine/trained-finder/tests/known-answers
```

Make them again whenever the made-up film generator changes the film
(its SHA-256 then no longer matches), or the weights change. The tests
fail until you do.
