# AASCE 2019 front-film labels

The local copy was inspected on 2026-11-10. The dataset root is
`boostnet_labeldata/`, containing `data/{training,test}/` and
`labels/{training,test}/`. Each labels folder has headerless
`filenames.csv`, `landmarks.csv` and `angles.csv`, plus one `.mat` per
film. The CSV rows correspond by position. The `.mat` files are not read.

| Checked item | Training | Test |
|---|---:|---:|
| JPEG films and rows in each CSV | 481 | 128 |
| `.mat` files | 481 | 128 |
| Width range, px | 355–1427 | 427–1386 |
| Height range, px | 973–3755 | 1159–2880 |
| Films with an order flag | 24 | 4 |
| Films with a top/bottom flag | 3 | 1 |
| Films with either of those flags | 25 | 5 |
| Films with left/right swaps | 2 | 0 |
| Films with an outside-image point | 1 | 0 |
| Films with any label flag | 27 | 5 |
| Films with a zero-length mid-line vector | 0 | 0 |
| Rows whose third Cobb angle is zero | 21 | 8 |

Every filename is unique within its split and resolves to a local film.
Every JPEG size header is baseline SOF0. Each filename row has one
column, each landmark row 136, and each angle row three. All numeric
values are finite. Column one is the largest angle on every row.

Landmarks use the first 68 columns for x and the remaining 68 for y.
Coordinates are fractions of the full image width or height; multiply
by W or H directly, with no half-pixel offset and no W−1/H−1 scaling.
Four consecutive points are image top-left, top-right, bottom-left,
bottom-right, for 17 vertebrae from T1 to L5. This interpretation agrees
with the image dimensions, top-to-bottom centres and paired endplates.
Corners retain their published ordering for Cobb reproduction; geometric
metrics canonicalise each endplate by x.

The native label audit reproduced all 609 published angle rows: maximum
absolute difference 0.013723 degrees, median per-film maximum difference
0.003181 degrees, and 602/609 films with every column within 0.01 degrees.
These meet the 0.02-degree maximum, 98-percent within 0.01 degrees and
0.005-degree median tolerances for the rounded landmarks. This checks the
Cobb implementation against the labels; it does not measure a finder.

The reader accepts CSV bytes and image dimensions. File lookup stays in
the command line; reports identify only the split and one-based row,
never the source filename. Unsafe path components, repeated names,
unequal row counts, malformed rows and non-finite numbers abort a run.
Finite coordinates outside the image remain in the table and are flagged.
An isolated order defect marks the one vertebra whose removal restores
order. Ambiguous defects mark both neighbours.

Primary corner, found and naming scores exclude flagged vertebrae.
Cobb uses the published three angles on every row, including flagged
films; reports also give the result without flagged films. The largest
angle is column one; the columns are not anatomical curve-region labels.

The current command can inspect JPEG dimensions and structure. Full pixel
decoding belongs to the film reader and must be added to preflight when
that reader becomes available. A valid header alone does not establish
that the compressed pixels decode successfully. Finder functions receive
the encoded bytes and are responsible for decoding them before scoring.

These are pre-cropped, upright, 8-bit front films without a scale or a
marker. They do not test orientation, patient left/right, skull, pelvis
or full-length behaviour. Errors are pixels and vertebral-height units,
never mm. The public test split is not the challenge's hidden test, so
its results cannot be compared with the challenge leaderboard. Accuracy
on generated films is not clinical accuracy.

Source layout: the `boostnet_labeldata` directory of the
`zlinzju/ke30_u7_AASCE2019` public mirror. No films, labels, weights or
source identifiers are stored here; tests construct their own tiny data.
