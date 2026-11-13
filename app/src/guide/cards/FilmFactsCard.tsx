import type { RowState } from '@/look-and-feel/data-attributes';
import { CARD_WORDS } from '@/words/guide';
import { CardRows } from './CardRows';
import { GuideCard } from './GuideCard';

/** One film's row: its name, and its two facts with where each came from. */
export interface FilmFactsCardFilm {
  id: string;
  name: string;
  /** `ok` when both facts were found, `you` when a person set one, `warn` while one waits for a person. */
  state: RowState;
  /** How big a pixel is, and which way the film faces, each with where it came from in plain words. */
  facts: readonly [size: string, facing: string];
}

interface FilmFactsCardProps {
  films: readonly FilmFactsCardFilm[];
}

/** Two facts per film: how big a pixel is and which way it faces, and where each fact came from. */
export function FilmFactsCard({ films }: FilmFactsCardProps) {
  const words = CARD_WORDS.filmFacts;
  const set = films.filter((film) => film.state === 'ok' || film.state === 'you').length;
  return (
    <GuideCard heading={words.heading} count={words.count(set, films.length)}>
      <CardRows rows={films.map((film) => ({ id: film.id, state: film.state, title: film.name, lines: film.facts }))} />
    </GuideCard>
  );
}
