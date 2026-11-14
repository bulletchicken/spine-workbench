// The stand-in's case file: the case and its films' PNGs as JSON text,
// so saving and reopening round-trip a case exactly. It is not the real
// `.spinecase` zip, which only the engine writes; the stand-in reads back
// only what it wrote, and refuses anything else as the engine refuses a
// file that is not a zip.

import type { OpenedCase } from '@/shapes';
import { EngineFailure } from '../engine-client';

const MARK = 'stand-in case file';
// Intake recognises cases by these bytes; this development payload is still not a zip.
const PREFIX = 'PK\u0003\u0004';

export function packCase(opened: OpenedCase): Uint8Array {
  const films = opened.films.map((film) => ({ kind: film.kind, png: toBase64(film.png) }));
  return new TextEncoder().encode(PREFIX + JSON.stringify({ mark: MARK, case: opened.case, films }));
}

export function unpackCase(bytes: ArrayBuffer): OpenedCase {
  let read: unknown;
  try {
    const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    read = JSON.parse(text.startsWith(PREFIX) ? text.slice(PREFIX.length) : text);
  } catch {
    throw notACase();
  }
  if (typeof read !== 'object' || read === null || (read as { mark?: unknown }).mark !== MARK) throw notACase();
  const { case: c, films } = read as { case: OpenedCase['case']; films: { kind: OpenedCase['films'][number]['kind']; png: string }[] };
  return { case: c, films: films.map((film) => ({ kind: film.kind, png: fromBase64(film.png) })) };
}

function notACase(): EngineFailure {
  return new EngineFailure({ code: 'case-not-a-zip', message: 'this is not a case file the stand-in wrote', detail: null });
}

const CHUNK = 0x8000;

function toBase64(bytes: Uint8Array): string {
  let text = '';
  for (let at = 0; at < bytes.length; at += CHUNK) text += String.fromCharCode(...bytes.subarray(at, at + CHUNK));
  return btoa(text);
}

function fromBase64(text: string): Uint8Array {
  return Uint8Array.from(atob(text), (letter) => letter.charCodeAt(0));
}
