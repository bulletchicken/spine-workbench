import type { RowState, SlotFace } from '@/look-and-feel/data-attributes';
import { CARD_WORDS } from '@/words/guide';
import { CardRows } from './CardRows';
import { GuideCard } from './GuideCard';

const STATE_OF_FACE: Record<SlotFace, RowState> = { empty: 'wait', reading: 'busy', hung: 'ok', bad: 'bad' };

/** One film's row: its name and the face of its slot. */
export interface FilmsCardFilm {
  /** The film's kind, or anything else that tells it from the others. */
  id: string;
  /** "Bending left". */
  name: string;
  face: SlotFace;
  /** Said in place of the usual line for this face, such as why this file could not be read. */
  line?: string | undefined;
}

interface FilmsCardProps {
  films: readonly FilmsCardFilm[];
}

/** The films and where each stands: waiting for a file, being read, ready, or unreadable. */
export function FilmsCard({ films }: FilmsCardProps) {
  const words = CARD_WORDS.films;
  const ready = films.filter((film) => film.face === 'hung').length;
  return (
    <GuideCard heading={words.heading} count={words.count(ready, films.length)}>
      <CardRows
        rows={films.map((film) => ({
          id: film.id,
          state: STATE_OF_FACE[film.face],
          title: film.name,
          lines: [film.line ?? words.lines[film.face]],
        }))}
      />
    </GuideCard>
  );
}
