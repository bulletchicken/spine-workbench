import { describe, expect, it } from 'vitest';
import { FILM_WIDTH, VIEWS } from './anatomy';
import { guessFor, pngGuessFor } from './guesses';

describe('what sensing each film answers', () => {
  it('gives three films a size from the file and bending right none', () => {
    expect(guessFor('front').scales.map((scale) => [scale.mmPerPx, scale.source])).toEqual([[0.45, { source: 'file', rule: 'geometry' }]]);
    expect(guessFor('side').scales).toHaveLength(1);
    expect(guessFor('bend-left').scales).toHaveLength(1);
    expect(guessFor('bend-right').scales).toEqual([]);
    expect(pngGuessFor('front').scales).toEqual([]);
  });

  it('turns only bending left, by two quarter turns, and faces the side film right', () => {
    expect(guessFor('front').upright.quarterTurns).toBe(0);
    expect(guessFor('bend-left').upright.quarterTurns).toBe(2);
    expect(guessFor('side').imageRight.imageRight).toBe('anterior');
  });

  it('finds the lead L on the image right of the front film, proposing and not deciding', () => {
    const { leadMarkers, imageRight } = guessFor('front');
    expect(leadMarkers.map((marker) => marker.letter)).toEqual(['l']);
    const box = leadMarkers[0]?.bounds;
    // Right of the spine at the marker's row, and on the film.
    expect(box?.left).toBeGreaterThan(VIEWS.front.X(150));
    expect(box?.right).toBeLessThan(FILM_WIDTH);
    expect(imageRight).toMatchObject({ imageRight: 'patient-left', certainty: 'medium', fileSays: null });
    expect(guessFor('bend-left').imageRight.imageRight).toBeNull();
  });

  it('hands out a fresh copy each time', () => {
    guessFor('front').scales.length = 0;
    expect(guessFor('front').scales).toHaveLength(1);
  });
});
