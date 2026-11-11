// Two files the app must refuse, and how.
//
// - A file that is not a film: a short text note, dropped by mistake. The
//   engine judges a film by its bytes alone, finds neither a PNG, a JPEG
//   nor a DICOM file, and refuses it with `film-unknown-format`.
// - A case that will not open: a real .spinecase zip whose case.json has a
//   field the format does not have, `notes`, because the format has no
//   place for free text. The strict reader refuses it with
//   `case-unknown-field`, naming the field's path, in the words the
//   engine's reader uses.

import type { EngineError } from '@/shapes';
import { writeZip } from './files/zip-file';
import { APP_VERSION } from './stages';

const utf8 = (text: string) => new TextEncoder().encode(text);

/** The bytes of a text note, which no film reader can read. */
export function notAFilm(): ArrayBuffer {
  return utf8('Bring the films on Tuesday.\nAsk about the brace.\n').buffer;
}

/** The case.json inside the refused case: canonical in every way but the one field too many. */
export const REFUSED_CASE_JSON = `{
  "format": "spine-case",
  "version": 1,
  "savedWith": {
    "app": "${APP_VERSION}"
  },
  "films": [],
  "marks": [],
  "nextVertebraId": 1,
  "notes": "Seen in clinic."
}
`;

const README = `This file is a Spine Workbench case.
See docs/contracts/case-file-v1.md in the project for its format.
`;

/** A case file that will not open, and the error that says why. */
export function refusedCase(): { file: ArrayBuffer; error: EngineError } {
  const zip = writeZip([
    { name: 'case.json', data: utf8(REFUSED_CASE_JSON) },
    { name: 'README.txt', data: utf8(README) },
  ]);
  return {
    file: zip.buffer,
    error: {
      code: 'case-unknown-field',
      message: 'unknown field `notes`, expected one of `format`, `version`, `savedWith`, `films`, `marks`, `nextVertebraId`',
      detail: 'notes',
    },
  };
}
