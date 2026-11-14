import type { CaseFileV1, FilmKind } from '@/shapes';
import type { FitSources } from '@/view-state/fit-sources';

/** Origin is not stored in the file: unknown standing marks use the wider trained model. */
export function reopenedSources(c: CaseFileV1, openings: Readonly<Record<FilmKind, number>>): FitSources {
  return Object.fromEntries(c.marks.filter((marks) => marks.vertebrae.length > 0).map(({ film }) => [film, {
    finder: film === 'front' || film === 'side' ? 'trained' : 'hand-built', opening: openings[film],
  }]));
}
