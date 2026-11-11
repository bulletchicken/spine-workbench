# Case-file known answers

All films and records here are made up. `verdicts.json` records the strict
JSON reader's answer, including the exact error code and JSON path.
`archive-verdicts.tsv` records the archive reader's answer as three columns:
file, `accept` or error code, and detail (empty means `null`). Rust and the
real WebAssembly engine read these same manifests.

The JSON refusals cover syntax and repeated keys, format and version,
unknown and missing fields, wrong types, film uniqueness and image bounds,
orphan and repeated marks, level order and template membership, both end
counts, L5 confirmation, every C7 consistency rule, blanked boxes, points,
scale bounds, unique ids, the next id and the app version. Every invalid
JSON also appears inside a ZIP, proving that opening a case uses the same
strict reader.

The archive refusals cover fixed entry names, path tricks, duplicate and
surplus entries, missing required entries and films, invalid ZIPs,
encryption, unsupported compression, local/central name disagreement,
bad CRCs, ZIP64 count limits, non-UTF8 JSON, entry size and compression limits, PNG dimensions
and significant bits, broken PNG data, the decoded-pixel budget and a
nonzero pixel inside a saved blanked box (including its unused low bits).

`make-fixtures.py` regenerates the additional JSON fixtures and all
archives using only Python's standard library, independently of the
engine's writer. Run it from any directory. The two accepted archives
contain the unchanged canonical `valid/minimal.json` and
`valid/four-films.json`. Their PNG rows repeat the four samples `0`, `1`,
`2^(bits-1)-1` and `2^bits-1`; saved blanked boxes are zero. ZIP timestamps
are fixed and PNGs contain only IHDR, sBIT, IDAT and IEND chunks.

The archive tests compare the complete records and decoded significant
samples. After one canonical save, a further pack/unpack is byte-for-byte
identical. Oversized declarations are patched into small archive headers
so the size checks can be exercised without allocating huge fixtures.
