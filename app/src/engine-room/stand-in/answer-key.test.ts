import type { FilmMarks, Px, VertebraOnFilm } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { compareWithAnswerKey } from './answer-key';
import { cornersOf, withCorners } from './quads';
import { smallStory } from './small-story';

const story = smallStory();
const front = story.pass.marks[0] as FilmMarks;
const key = { ...story.madeUp.spine.answerKey, films: story.madeUp.spine.answerKey.films.slice(0, 1) };

/** The vertebra with every corner moved by `by`. */
function moved(vertebra: VertebraOnFilm, by: Px): VertebraOnFilm {
  const corners = cornersOf(vertebra).map((corner) => ({ ...corner, at: [corner.at[0] + by[0], corner.at[1] + by[1]] as Px }));
  return withCorners(vertebra, corners as ReturnType<typeof cornersOf>);
}

describe('comparing marks with the answer key', () => {
  it('leaves a true vertebra unmatched when no marked one is near it', () => {
    // L5 left out: the nearest marked vertebra, L4, is 70 px away, past half of L5's 54 px height.
    const comparison = compareWithAnswerKey([{ ...front, vertebrae: front.vertebrae.slice(0, 2) }], key);
    const film = comparison.films[0];
    expect(film).toMatchObject({ expected: 3, found: 2, levelsRight: 2, meanCornerErrorPx: 5 });
    expect(film?.vertebrae[2]).toEqual({ level: 'L5', vertebraId: null, levelRight: false, meanCornerErrorPx: null, maxCornerErrorPx: null });
  });

  it('matches within half the true height, and no further', () => {
    // Each true corner lies (3, 4) from its mark. Moving L3's marks by (3, −22) leaves its truth (0, 26) away:
    // 26 px, inside half of 54. By (3, −24) it is 28 px away, outside.
    const near = compareWithAnswerKey([{ ...front, vertebrae: [moved(front.vertebrae[0] as VertebraOnFilm, [3, -22]), ...front.vertebrae.slice(1)] }], key);
    expect(near.films[0]?.vertebrae[0]).toMatchObject({ vertebraId: 0, meanCornerErrorPx: 26, maxCornerErrorPx: 26 });
    const far = compareWithAnswerKey([{ ...front, vertebrae: [moved(front.vertebrae[0] as VertebraOnFilm, [3, -24]), ...front.vertebrae.slice(1)] }], key);
    expect(far.films[0]?.vertebrae[0]).toMatchObject({ vertebraId: null });
  });
});
