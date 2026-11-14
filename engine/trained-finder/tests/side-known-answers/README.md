# Side known answers

These two answers come from artificial byte arrays, never films.
`finder-prep/make_side_known_answers.py` records them. Its README
gives the explicit commands. No build runs the script.

`synthetic.json` uses a 128 by 128 input. Generate row-major bytes with
xorshift32, state 20261113. For every next value, xor left 13, xor
right 17, xor left 5, masking to u32 after each operation. Take the
top byte. Divide each byte by 255 as float32, then subtract 0.5 once.

The synthetic weights use the same generator, seeded once at 17.
Visit the 36 tensors in packet order: e1, e2, e3, e4, e5, d4, d3,
d2, with convolution 0 then 3 and weight then bias in each block;
then landmarks.0 and landmarks.2, weight then bias. Every weight is
`((next >> 24) - 128) * 2^-14`. Every bias is zero and consumes no
stream value. BN is removed, giving identity normalisation. No large
synthetic weight file is kept.

`trained.json` uses the same artificial-input rule at 512 by 384 and
the fixed checkpoint, with its original unfused BN in CPU evaluation
mode. Its network comparison belongs only to a build containing
the optional side packet. It records no patient information.

Both JSON files give the source hashes, software versions, input
hashes, all 18 convolution activation shapes, statistics, selected
CHW row-major samples and all 20 decoded points in level-major
L1–L5, supA/supP/infA/infP order. Block samples follow BN and ReLU;
landmarks.0 follows ReLU; landmarks.2 is the raw logit output.
Statistics use float64. Points use input pixel centres, with
`((cell + 0.5) * 4 - 0.5)`, without clipping. The reference forward
receives uncentred floats and subtracts 0.5 internally.

Each sampled activation allows `2e-4 + 1e-4 * abs(reference)` error.
Each decoded point allows 0.01 input pixels. These are numerical
port tolerances, not accuracy or confidence claims. Resize bytes
must match exactly. Small drawn ramps and checkerboards pin Pillow
13.0.0 bilinear resizing, unchanged axes, shrinking, enlarging,
odd padding and both directions of ties-to-even rounding. Their
affines retain independently rounded horizontal and vertical scales.

`finder-prep/check_side_preparation.py` checks the synthetic network
without the optional packet or reference directory. Hand-worked
layer and decoder boundary cases belong to the Rust tests.
