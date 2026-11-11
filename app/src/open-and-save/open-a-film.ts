// Opens one film in the engine and keeps what the bench needs: the film's
// pixels for the screen and its thumbnail. The one place a film is opened,
// for step 1 and for reopening a case alike.
//
// Opening a kind that is already open replaces that film. If two openings
// of one kind overlap, only the newest one keeps its pixels: an older one
// that finishes later must not put its film back on the screen. The older
// one rejects with `pass-cancelled`, as the engine's own cut-off calls do,
// whether it was overtaken while the engine read the bytes or while it
// handed back the pixels.
//
// If the pixels fail after the engine has opened the film, the engine holds
// the new film, this slot keeps no pixels, its count is not raised, and the
// case still holds the record it held before. The caller must then either
// open the film again or retire the slot in the engine and in the case, so
// the case, the pixels and the engine describe the same film once more.
//
// Overlapping openings of one slot can leave the same split. Say film O is
// open, opening A's bytes are read, and opening B of the same slot starts:
// the engine cuts A's pixels off with `pass-cancelled`, and then refuses
// B's bytes. The engine keeps A, the last film it opened, while the slot
// keeps O's pixels and the case keeps O's record. So a caller opens one
// slot's films one after another, never two at once; and a caller that
// does overlap them must, when the newer opening is refused, open that
// slot again or retire it, as after a pixel failure.
//
// Which opening is newest is counted here, for the module, not for each
// film-pixels store: two stores used side by side would cancel each
// other's openings of the same kind. The app has one store.

import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { filmPixels, type FilmPixelsApi } from '@/case-state/film-pixels';
import type { Film, FilmKind } from '@/shapes';

/** What the engine says of a film just read: its facts and its thumbnail. */
export type OpenedFilm = Awaited<ReturnType<EngineClient['openFilm']>>;

/** The engine calls opening a film needs. A full EngineClient satisfies it. */
export type FilmOpener = Pick<EngineClient, 'openFilm' | 'displayPixels'>;

// The newest opening started for each kind, by a number that only grows.
const newestOpening: Record<FilmKind, number> = { front: 0, side: 0, 'bend-left': 0, 'bend-right': 0 };
let openingsStarted = 0;

/** The failure an opening that a newer one replaced rejects with. */
function replaced(): EngineFailure {
  return new EngineFailure({ code: 'pass-cancelled', message: 'a newer opening of this film replaced this one', detail: null });
}

/**
 * Open `bytes` as the film of `kind`, with its saved record or none. On
 * success the film's pixels and thumbnail are kept in `films` and the
 * slot's count of openings goes up by one. If the engine refuses the
 * bytes, the film that was open stays open and `films` is left as it was.
 * An opening that a newer one of the same kind replaced rejects with
 * `pass-cancelled`, and keeps nothing.
 */
export async function openAFilm(
  engine: FilmOpener,
  kind: FilmKind,
  bytes: ArrayBuffer,
  saved: Film | null,
  films: FilmPixelsApi = filmPixels,
): Promise<OpenedFilm> {
  openingsStarted += 1;
  const ticket = openingsStarted;
  newestOpening[kind] = ticket;

  const opened = await engine.openFilm(kind, bytes, saved);
  if (newestOpening[kind] !== ticket) throw replaced();
  let pixels: Uint16Array;
  try {
    pixels = await engine.displayPixels(kind);
  } catch (error) {
    // The engine has replaced this film, so the pixels kept for the slot
    // are no longer its film's. Let them go, unless a newer opening has
    // started since: the slot is then that opening's to fill or leave.
    if (newestOpening[kind] === ticket) films.getState().retired(kind);
    throw error;
  }
  if (newestOpening[kind] !== ticket) throw replaced();
  films.getState().opened(kind, { pixels, thumbnail: opened.thumbnail });
  return opened;
}
