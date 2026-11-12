# The trained finder's weights

The numbers the trained finder runs on. They were learned once, from
labelled X-ray films, and are compiled into the engine; nothing here is
learned or downloaded at run time.

## The files

| File | Holds |
|---|---|
| `finder-weights.bin` | every tensor of both networks as little-endian float32, end to end, in row-major order |
| `finder-weights.json` | each tensor's name, shape and place in the `.bin`; the score threshold and the corner blend; the SHA-256 of the other three files and of the two training checkpoints they were packed from |
| `chain-priors.json` | the chain's learned priors for each level from T1 to L5, byte for byte as fitted |
| `framing.json` | the seven settings of the framing rule, byte for byte as chosen |

Read `finder-weights.json` for the exact sizes and SHA-256s; the crate
checks them when it reads the weights. `finder-prep/pack_weights.py`
writes all four.

## The two networks

**The main network** (2,625,987 weights) reads the whole film shrunk to
384 by 768. Five encoder blocks, each two 3×3 convolutions with batch
norm and ReLU, the first at stride 2, widen it to 24, 48, 96, 160 and
224 channels while halving it each time. Three decoder blocks upsample
by two (bilinear), join the matching encoder map and narrow it to 160,
96 and 64 channels, ending at 96 by 192. Three heads, each a 3×3 and
then a 1×1 convolution, give one map of centre scores, eight of corner
offsets and two of where each centre sits in its cell. Its batch norms
are folded into the convolutions before them, in float64 and rounded
to float32 once, so the file holds 2,624,243 of its numbers.

**The corner network** (93,812 weights) reads one 128 by 128 patch cut
around a vertebra, averaged down to 64 by 64, with the vertebra's
current four corners. Three encoder blocks of 16, 32 and 48 channels
and two decoder blocks of 32 and 16, each two 3×3 convolutions with
group norm and SiLU, end in a 1×1 convolution giving one map for each
corner; each corner's new place is the map's weighted average place.
Group norm depends on the picture and cannot be folded, so its tensors
are stored as trained.

## What they were trained on

The 481 labelled front films of SpineWeb Dataset 16, as released for
the AASCE 2019 challenge: front spine films showing scoliosis, from one
hospital's collection, cropped to the spine, each vertebra from T1 to
L5 marked by its four corners. No pretrained weights were used.

- **The main network** learned from 433 of the 481 films. The other 48,
  chosen at random, not by child, were held back to choose the
  checkpoint and the score threshold. It trained for 120 passes over
  the films, eight at a time, with AdamW and a one-cycle learning rate
  peaking at 0.002. Each film was randomly turned up to 8 degrees,
  zoomed, shifted, mirrored and changed in exposure, with a little
  noise. It learned a heat map of vertebra centres (a focal loss) and,
  at each centre, the four corners and the centre's place in its cell;
  the kept checkpoint is from pass 90.
- **The chain priors and the corner network** were fitted on 373 of the
  481 films, from 174 children; the other 108 films, from 46 other
  children, were kept to judge them. The main network had trained on 97
  of those 108, so they judge the later parts, not the network.

Training code is not part of this repository.

## How they were tested

Everything (code, settings, weights) was frozen and its hashes recorded
before the test films or their labels were opened, and the test was run
once. The figures are for 112 of the release's 128 test films: those
from children with none of their films among the 481. A separate
re-score, with its own reading and scoring code, gave the same counts.
The numbers are in `../README.md`.

## Credit

The films' source asks that any publication resulting from any use of
the films cite:

> Wu, H., Bailey, C., Rasoulinejad, P., and Li, S. (2017). Automatic
> landmark estimation for adolescent idiopathic scoliosis assessment
> using BoostNet. Medical Image Computing and Computer Assisted
> Intervention (MICCAI 2017), pages 127 to 135.
> https://doi.org/10.1007/978-3-319-66182-7_15

The same credit is in `NOTICE`.

## What the films' pages say about use

The dataset's own page, read from archived copies because the site is
now offline, states one condition: the citation above, for any
publication resulting from any use of the films. Access was by a
request form naming an institution and a lead researcher; the form
shows no agreement text. No wording was found, there or on the
challenge's public pages or in the two papers describing the films, on
research-only or commercial use, on sharing the films, or on models or
weights trained on them. The challenge's own data page could not be
read. The films themselves are not in this repository.
