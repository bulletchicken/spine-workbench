import { describe, expect, it } from 'vitest';
import { FILM_KINDS } from '@/case-state/selectors';
import { assignPlaces } from './assign-places';

describe('assigning film places', () => {
  it('gives a named place the first file, then takes empty places in order', () => {
    expect(assignPlaces(3, 'side', [])).toEqual({ places: ['side', 'front', 'bend-left'], leftOut: 0 });
    expect(assignPlaces(3, 'side', ['front', 'side'])).toEqual({ places: ['side', 'bend-left', 'bend-right'], leftOut: 0 });
  });
  it.each([0, 1, 2, 3, 4])('fills the remaining places after %i are full', (full) => {
    expect(assignPlaces(6, null, FILM_KINDS.slice(0, full))).toEqual({ places: FILM_KINDS.slice(full), leftOut: full + 2 });
  });
  it('takes no place for no files, and replaces a full place only when named', () => {
    expect(assignPlaces(0, 'front', [])).toEqual({ places: [], leftOut: 0 });
    expect(assignPlaces(2, 'front', FILM_KINDS)).toEqual({ places: ['front'], leftOut: 1 });
    expect(assignPlaces(2, null, FILM_KINDS)).toEqual({ places: [], leftOut: 2 });
  });
});
