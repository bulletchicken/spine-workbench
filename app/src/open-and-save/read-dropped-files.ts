// Tells the files a person gives the app apart by their first four bytes
// alone. A case file is a zip, which begins with the zip signature; a film
// is anything else, and the engine judges a film by its bytes later. A
// file's name is never read: the input is blobs, which carry no name, so
// nothing here can look at one. Nor is it kept: a File given in is kept as
// a plain Blob over the same data, which has no name at all.
//
// Each file comes back as one entry, in the order it was given: a film, a
// case file, or a refusal naming the fault code that words it in
// words/faults.ts. Files are held as blobs, not bytes, so a large film is
// read in full only when it is opened, by read-bytes.ts. A file is not read
// in full here.

import type { ErrorCode } from '@/shapes';

const MiB = 1024 * 1024;

/** Most a film may be, in bytes: the engine's intake limit (`MAX_FILE`, 128 MiB). */
export const MAX_FILM_BYTES = 128 * MiB;

const CASE_FILMS = 4;
const MAX_PIXELS = 40_000_000;
const BYTES_PER_PIXEL = 2;
const SAVE_ALLOWANCE = 1.1;
/**
 * Most a case file may be, in bytes. A case holds at most four films, each
 * saved as a PNG of at most `MAX_PIXELS` pixels at two bytes a pixel. A
 * noisy 16-bit PNG barely compresses, so the allowance is 10% over the raw
 * pixels, which covers the PNG headers, the zip and the case record.
 */
export const MAX_CASE_BYTES = Math.ceil(CASE_FILMS * MAX_PIXELS * BYTES_PER_PIXEL * SAVE_ALLOWANCE);

export interface ReadLimits {
  filmBytes: number;
  caseBytes: number;
}

export const READ_LIMITS: ReadLimits = { filmBytes: MAX_FILM_BYTES, caseBytes: MAX_CASE_BYTES };

/** One file given to the app, in the order it was given. */
export type DroppedFile =
  | { kind: 'film'; blob: Blob }
  | { kind: 'case'; blob: Blob }
  | { kind: 'refused'; code: ErrorCode };

const ZIP_SIGNATURE = [0x50, 0x4b, 0x03, 0x04];

function isZip(head: Uint8Array): boolean {
  return head.length === ZIP_SIGNATURE.length && ZIP_SIGNATURE.every((byte, index) => head[index] === byte);
}

/** Judge one blob by its size and its first four bytes. */
async function judge(blob: Blob, limits: ReadLimits): Promise<DroppedFile> {
  // An empty file has lost its data on the way, as a cut-off copy does.
  if (blob.size === 0) return { kind: 'refused', code: 'film-truncated' };
  let head: Uint8Array;
  try {
    head = new Uint8Array(await blob.slice(0, ZIP_SIGNATURE.length).arrayBuffer());
  } catch {
    // The system would not give the page this file: it moved or changed after it was given.
    return { kind: 'refused', code: 'film-truncated' };
  }
  // A slice of the whole is a Blob, never a File, so it carries no name;
  // it is still read only when asked.
  const nameless = blob.slice(0, blob.size);
  if (isZip(head)) {
    return blob.size > limits.caseBytes ? { kind: 'refused', code: 'case-too-large' } : { kind: 'case', blob: nameless };
  }
  return blob.size > limits.filmBytes ? { kind: 'refused', code: 'film-too-large' } : { kind: 'film', blob: nameless };
}

/**
 * Judge what was given, one file at a time. Each file that cannot be read
 * is refused on its own; the others are still judged.
 */
export async function readDroppedFiles(blobs: readonly Blob[], limits: ReadLimits = READ_LIMITS): Promise<DroppedFile[]> {
  const out: DroppedFile[] = [];
  for (const blob of blobs) {
    out.push(await judge(blob, limits));
  }
  return out;
}
