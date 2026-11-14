# Placements are compared over versions

**Decided:** the load bench never gives a placement a single number.
Each placement is run on the same set of versions of the spine (32 by
default), and placements are compared with each other by paired counts
and ranges: "lighter on its screws in 26 of 32 versions", "curve 12° to
18° in 8 of 10 versions". The placement proposals count their level
choices and screw sizes over the same versions.

## Why

- Films cannot show how stiff a spine is. A stiffness can only be worked
  out from a film taken under a load of known size, and no spine film
  has one. Published simulators that reach a few degrees of curve error
  do it with gains fitted to their own patients. A single force or
  stress per placement would therefore be invented precision.
- A version is this spine with one draw of everything the films cannot
  fix: the fit's noise in each vertebra's place and angles, pedicle
  widths, tissue stiffness, the rib cage's share, bone strength, rod
  yield, weight and steps a day. Each draw comes from a published range
  or a stated prior, so the spread of answers shows what the films leave
  open.
- Forces scale with the unknown stiffness, but which part carries more
  than another, and which of two placements loads its screws less, can
  hold across the versions even when no single force is known. Paired
  counts show exactly that: two placements meet the same stiffness, the
  same bone and the same weight in each version, so a count can be clear
  when two ranges overlap.

## Consequences

- One version sampler, in `implant-layout`, serves both crates, and
  version k is the same spine in every batch and in both crates given
  the same fit, seed and number of versions.
- Every estimate carries a tag: set by the films (narrow whatever the
  unknowns), comparison only (wide, but its difference from the first
  placement keeps its sign in nearly every version), or assumed. Absolute
  forces and counts past published strength are always assumed.
- A count of 32 versions has a standard error of up to about 0.09, so a
  difference is called clear only at 24 of 32 or more, with a margin
  below which two values count as tied.
- The bench runs in short batches of a few versions each, so no call
  holds the one spine worker for long.
- Fit noise is drawn independently per vertebra, because the fit gives
  each vertebra's spread but not how neighbours move together. That
  widens the joint noise; it narrows when the fit gives its covariance.
- The answers live on screen only; a placement is not saved with the
  case.
