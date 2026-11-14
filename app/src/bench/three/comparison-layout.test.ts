import { expect, it } from 'vitest';
import { comparisonLayout } from './comparison-layout';
it('keeps equal film and spine halves, reserving the wide rail and fitting the whole film box', () => {
  const wide = comparisonLayout(1280, 672, 90, 24, 24, 820, 28);
  expect(wide.film.height).toBe(644); expect(wide.film.width).toBe(322);
  const filmCentre = wide.film.x + wide.film.width / 2;
  expect(filmCentre + wide.shift).toBe(1280 - filmCentre);
  const narrow = comparisonLayout(608, 572, 0, 24, 24, 820, 28);
  expect(narrow.film.x + narrow.film.width / 2).toBe(152);
  expect(narrow.shift).toBe(304);
});
