# Films are saved as lossless PNG

**Decided:** every film in a case file is stored as `films/<kind>.png`, a
lossless greyscale PNG at the film's own bit depth (left-aligned in an 8-
or 16-bit PNG, with an `sBIT` chunk saying how many bits are real),
re-encoded from the decoded pixels. The
original DICOM, JPEG or PNG bytes are never stored.

## Why

- **Nothing to scrub.** An original DICOM carries names, identifiers and
  dates; a JPEG may carry EXIF. A PNG written from pixels carries none of
  them, so nothing can leak, and there is no scrubber to get wrong.
- **One format to read back.** A reopened case goes through the same PNG
  path as any film, with no codec to support twice.
- **Lossless at the film's depth.** A 12-bit film stays 12-bit; stored
  values shift back exactly. With the value map in the record (offset,
  inversion, modality map), windows from the original file still apply.

## Consequences

- A saved case can be larger than a compressed original. That is the
  price of carrying no metadata.
- What reading noticed that the PNG no longer shows (lossy source,
  multi-frame file, burnt-in annotation) is kept as intake flags on the
  film record, and the source format is recorded.
- Boxes the user blanked are zero in the stored PNG.
