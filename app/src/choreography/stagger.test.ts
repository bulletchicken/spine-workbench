// Places in a cascade, counted by hand: with six levels and an edit at the
// third (index 2), the edit is 0, its neighbours 1, and the far end 3.

import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { placesFromEdit, placeStyle } from './stagger';

describe('placesFromEdit', () => {
  it('counts outward from the edited level', () => {
    expect(placesFromEdit(6, 2)).toEqual([2, 1, 0, 1, 2, 3]);
  });

  it('counts down the spine from an edit at the top, and up it from one at the bottom', () => {
    expect(placesFromEdit(4, 0)).toEqual([0, 1, 2, 3]);
    expect(placesFromEdit(4, 3)).toEqual([3, 2, 1, 0]);
  });

  it('has nothing to place when there are no levels', () => {
    expect(placesFromEdit(0, 0)).toEqual([]);
  });

  // An edit that took away the level above the first of three, and one two
  // past the last: counted out by hand.
  it('counts from an edit that lies past either end', () => {
    expect(placesFromEdit(3, -1)).toEqual([1, 2, 3]);
    expect(placesFromEdit(3, 4)).toEqual([4, 3, 2]);
  });

  it('refuses an edit between two levels, which would give places CSS cannot count', () => {
    expect(() => placesFromEdit(3, 1.5)).toThrow(RangeError);
    expect(() => placesFromEdit(3, Number.NaN)).toThrow(RangeError);
  });

  it('puts the edit first and moves one place per level away from it', () => {
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 40 }), fc.nat(), (count, seed) => {
        const editedAt = seed % count;
        const places = placesFromEdit(count, editedAt);
        expect(places).toHaveLength(count);
        expect(places[editedAt]).toBe(0);
        expect(places.filter((place) => place === 0)).toHaveLength(1);
        for (let level = 1; level < count; level += 1) {
          expect(Math.abs((places[level] ?? Number.NaN) - (places[level - 1] ?? Number.NaN))).toBe(1);
        }
      }),
    );
  });
});

describe('placeStyle', () => {
  it('carries a place to CSS as --i', () => {
    expect(placeStyle(3)).toEqual({ '--i': 3 });
  });
});
