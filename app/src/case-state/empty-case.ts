// The case a session starts with, before any film: no films, no marks.

import { CASE_FORMAT, CASE_VERSION, type CaseFileV1 } from '@/shapes';

/**
 * This app's version, as a case file records it in `savedWith.app`. The
 * app and the engine are built from one repository and share the
 * workspace version in Cargo.toml; empty-case.test.ts keeps the two
 * equal.
 */
export const APP_VERSION = '0.1.0';

/** A case with nothing in it yet. */
export function emptyCase(): CaseFileV1 {
  return { format: CASE_FORMAT, version: CASE_VERSION, savedWith: { app: APP_VERSION }, films: [], marks: [], nextVertebraId: 1 };
}
