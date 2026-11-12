import { FILM_KINDS } from '@/case-state/selectors';
import type { FilmKind } from '@/shapes';

/** A named place takes the first film, then the empty places take the rest. */
export function assignPlaces(files: number, onPlace: FilmKind | null, filled: readonly FilmKind[]): { places: FilmKind[]; leftOut: number } {
  const places: FilmKind[] = files > 0 && onPlace !== null ? [onPlace] : [];
  for (const kind of FILM_KINDS) {
    if (places.length >= files) break;
    if (!places.includes(kind) && !filled.includes(kind)) places.push(kind);
  }
  return { places, leftOut: Math.max(0, files - places.length) };
}
