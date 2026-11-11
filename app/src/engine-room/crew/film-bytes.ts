// The bytes of each film the crew holds open, for this session only and
// never saved: what a respawned film worker reads again, so a crash costs
// a moment and nothing else.
//
// A film's entry changes only when an `open-film` for it answers: a film
// that could not be read leaves the open one's bytes in place, exactly as
// its worker leaves the open film in place.

import type { Film, FilmKind } from '@/shapes';

export interface KeptFilm {
  bytes: ArrayBuffer;
  saved: Film | null;
}

export class FilmBytes {
  #kept = new Map<FilmKind, KeptFilm>();

  keep(kind: FilmKind, film: KeptFilm): void {
    this.#kept.set(kind, film);
  }

  get(kind: FilmKind): KeptFilm | undefined {
    return this.#kept.get(kind);
  }

  drop(kind: FilmKind): void {
    this.#kept.delete(kind);
  }
}
