// Worked by hand on the sample case's films: 0.45 mm a pixel where a film
// has a size, so the point (400, 1000) is 180 mm across and 450 mm down.

import { describe, expect, it } from 'vitest';
import { emptyCase } from '@/case-state/empty-case';
import { caseAt } from '@/sample-case/stages';
import type { CaseFileV1 } from '@/shapes';
import { pixelSizeShown, pixelSizeWritten, positionOf } from './status-readings';

const screen = [0, 0] as const;

describe('where the pointer is', () => {
  const sized = caseAt('films-all-set').case;
  const unsized = caseAt('four-films-up').case;

  it('is in whole millimetres on a film with a size', () => {
    expect(positionOf({ film: 'front', at: [400, 1000], screen }, sized)).toEqual({ x: 180, y: 450, unit: 'mm' });
  });

  it('is in whole pixels, and says so, on a film with no size', () => {
    expect(unsized.films.find((film) => film.kind === 'bend-right')?.geometry.mmPerPx).toBeNull();
    expect(positionOf({ film: 'bend-right', at: [400.4, 999.6], screen }, unsized)).toEqual({ x: 400, y: 1000, unit: 'px' });
  });

  it('uses the vertical size when pixels are not square', () => {
    const tall: CaseFileV1 = {
      ...sized,
      films: sized.films.map((film) => (film.kind === 'front' ? { ...film, geometry: { ...film.geometry, mmPerPx: 0.5, mmPerPxVertical: 0.25 } } : film)),
    };
    expect(positionOf({ film: 'front', at: [100, 100], screen }, tall)).toEqual({ x: 50, y: 25, unit: 'mm' });
  });

  it('is nothing when the pointer is on no film, or on one the case does not hold', () => {
    expect(positionOf(null, sized)).toBeNull();
    expect(positionOf({ film: 'front', at: [1, 1], screen }, emptyCase())).toBeNull();
  });
});

describe('the pixel size shown', () => {
  const sized = caseAt('films-all-set').case;
  const unsized = caseAt('four-films-up').case;

  it('is written with two or three decimals', () => {
    expect(pixelSizeWritten(0.45)).toBe('0.45');
    expect(pixelSizeWritten(0.144)).toBe('0.144');
    expect(pixelSizeWritten(0.2)).toBe('0.20');
    expect(pixelSizeWritten(0.1514)).toBe('0.151');
  });

  it('is that of the film under the pointer, or nothing when that film has none', () => {
    expect(pixelSizeShown(sized, 'side')).toBe('0.45');
    expect(pixelSizeShown(unsized, 'bend-right')).toBeNull();
  });

  it('with the pointer on no film, is the one size the sized films share', () => {
    expect(pixelSizeShown(sized, null)).toBe('0.45');
    // A film still waiting for its size does not hide the size the others share.
    const oneWaiting: CaseFileV1 = {
      ...sized,
      films: sized.films.map((film) => (film.kind === 'bend-right' ? { ...film, geometry: { ...film.geometry, mmPerPx: null } } : film)),
    };
    expect(pixelSizeShown(oneWaiting, null)).toBe('0.45');
    // Nothing is known about four films just hung, and an empty case has no film at all.
    expect(pixelSizeShown(unsized, null)).toBeNull();
    expect(pixelSizeShown(emptyCase(), null)).toBeNull();
  });

  it('is nothing when the films differ and the pointer is on none of them', () => {
    const mixed: CaseFileV1 = {
      ...sized,
      films: sized.films.map((film) => (film.kind === 'side' ? { ...film, geometry: { ...film.geometry, mmPerPx: 0.3 } } : film)),
    };
    expect(pixelSizeShown(mixed, null)).toBeNull();
    expect(pixelSizeShown(mixed, 'side')).toBe('0.30');
  });
});
