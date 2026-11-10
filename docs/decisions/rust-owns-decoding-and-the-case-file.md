# Rust owns decoding and the case file

**Decided:** films are decoded in Rust (`engine/film-reader`), inside the
film worker that keeps them, and case files are read and written only by
Rust (`engine/shapes` for `case.json`, `engine/case-file` for the zip).
TypeScript builds a `CaseFileV1` object; Rust checks it before anything
is written.

## Why

- **Privacy happens where reading happens.** DICOM tags not on the
  keep-list are dropped while the file is parsed, and the shapes have no
  field that could hold them. One decoder in one language is one place
  to check.
- **One decode path.** A dropped file, a made-up film and a film from a
  saved case all enter through `FilmDesk.open`, so a reopened case reads
  exactly the pixels it saved.
- **One strict reader.** The case file is read in one language, against
  the same shapes that generate the TypeScript and the JSON Schema. A
  second reader in TypeScript could drift; a schema library and a zip
  library would be two more dependencies doing what Rust already does.
- The same code runs natively, so the command line and the tests read
  and write the very files the app does.

## Consequences

- The app holds no DICOM, JPEG or zip code, and never needs a file's
  name: files are recognised by their bytes.
- A value from the app that does not match the shapes (a wrong type, an
  unknown field, a field left out) is refused as `engine-out-of-sync`.
- The golden case files in `proving-ground/known-answers/case-files/` are
  read by Rust natively and through the real engine in Node, against one
  list of verdicts.
