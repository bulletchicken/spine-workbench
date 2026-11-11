import { describe, expect, it } from 'vitest';
import { guessFor, pngGuessFor } from './guesses';
import { keyframesOnly } from './keyframes-only';
import { passMarks } from './marks';
import story from './story';

describe('the story the stand-in answers from', () => {
  it('hangs the four films, each with what sensing it answers', () => {
    for (const kind of ['front', 'side', 'bend-left', 'bend-right'] as const) {
      expect(story.films[kind].guess).toEqual(guessFor(kind));
      expect(story.films[kind].facts.kind).toBe(kind);
    }
  });

  it('plays the keyframes of the pass and leaves its marks, ids from 0', () => {
    expect(story.pass.marks).toEqual(passMarks());
    expect(story.pass.frames).toEqual(keyframesOnly(passMarks()));
    expect(story.pass.warnings).toEqual([]);
    expect(story.pass.twoFilms?.disagree).toBe(false);
    expect(story.pass.twoFilms?.residuals.map((residual) => residual.level)).toEqual(story.pass.marks[0]?.vertebrae.map((vertebra) => vertebra.level));
  });

  it('makes up the same spine as PNGs, bending right\'s the very file it hangs', () => {
    const films = story.madeUp.spine.films;
    expect(films.map((film) => film.kind)).toEqual(['front', 'side', 'bend-left', 'bend-right']);
    expect(films[3]?.png).toEqual(new Uint8Array(story.films['bend-right'].file));
    expect([...(films[0]?.png.subarray(1, 4) ?? [])]).toEqual([80, 78, 71]);
    expect(story.madeUp.films.front.facts.source).toBe('png');
    expect(story.madeUp.films.front.guess.scales).toEqual([]);
    expect(story.madeUp.spine.answerKey.films).toHaveLength(4);
  });

  it('hands every reader its own copy, so changing one leaves the next whole', () => {
    story.films.front.guess.scales.length = 0;
    expect(story.films.front.guess).toEqual(guessFor('front'));
    story.madeUp.films['bend-left'].guess.upright.quarterTurns = 0;
    expect(story.madeUp.films['bend-left'].guess).toEqual(pngGuessFor('bend-left'));
    story.pass.marks.length = 0;
    expect(story.pass.marks).toHaveLength(4);
    story.madeUp.spine.answerKey.films.length = 0;
    expect(story.madeUp.spine.answerKey.films).toHaveLength(4);
  });

  it('refuses the case that will not open', () => {
    expect(story.refusedCase.error.code).toBe('case-unknown-field');
    expect(story.refusedCase.file.byteLength).toBeGreaterThan(0);
  });
});
