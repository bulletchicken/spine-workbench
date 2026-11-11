import { describe, expect, it } from 'vitest';
import { filmStamp } from './film-stamp';
import { testFilm } from './test-frames';

describe('a film’s stamp', () => {
  const stamp = filmStamp(testFilm('front'), 1);

  it('stays the same for the same film and opening', () => {
    expect(filmStamp(testFilm('front'), 1)).toBe(stamp);
  });

  it('changes when the film is turned', () => {
    const turned = testFilm('front');
    turned.geometry.quarterTurns = 2;
    expect(filmStamp(turned, 1)).not.toBe(stamp);
  });

  it('changes when the film is opened again or replaced', () => {
    expect(filmStamp(testFilm('front'), 2)).not.toBe(stamp);
  });

  it('changes when the brightness is flipped', () => {
    const flipped = testFilm('front');
    flipped.values = { ...flipped.values, inverted: true, invertedByUser: true };
    expect(filmStamp(flipped, 1)).not.toBe(stamp);
  });

  it('changes when a box is blanked, or a blanked box moves', () => {
    const blanked = testFilm('front');
    blanked.blanked = [{ left: 10, top: 10, right: 60, bottom: 30 }];
    const moved = testFilm('front');
    moved.blanked = [{ left: 10, top: 10, right: 60, bottom: 31 }];
    expect(filmStamp(blanked, 1)).not.toBe(stamp);
    expect(filmStamp(moved, 1)).not.toBe(filmStamp(blanked, 1));
  });

  it('changes for another film kind in the same opening', () => {
    expect(filmStamp({ ...testFilm('front'), kind: 'side' }, 1)).not.toBe(stamp);
  });

  it('stays the same when only the facing, the scale or the confirmations change', () => {
    const faced = testFilm('front');
    faced.geometry = { ...faced.geometry, imageRight: 'patient-left', mmPerPx: 0.2, confirmed: { ...faced.geometry.confirmed, leftRight: true, scale: true } };
    expect(filmStamp(faced, 1)).toBe(stamp);
  });
});
