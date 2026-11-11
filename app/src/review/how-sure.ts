// How sure the engine is about one vertebra on one film, worked out from the
// case alone. Four of the design's states, plus "not counted" for anything
// outside the film's count.

import { TEMPLATE_LEVELS, compareLevels } from '@/shapes';
import type { ConfidenceBand, FilmMarks, Mark, MarkSource, PedicleMark, VertebraMarks, VertebraOnFilm } from '@/shapes';
import type { Conf } from '@/look-and-feel/data-attributes';

/** A mark as the rules read it: who placed it last, and whether it is pinned. */
interface Handle {
  source: MarkSource;
  pinned: boolean;
}

/** Sources a person put on a mark. A boundary wand's moves do not count. */
const PERSON_SOURCES: ReadonlySet<MarkSource> = new Set<MarkSource>(['hand', 'pin', 'region-wand']);

const BAND_STATE: Record<ConfidenceBand, Conf> = { high: 'sure', medium: 'look', low: 'needs' };

/** Most to least urgent: the first one any counted film shows wins. */
const URGENCY: readonly Conf[] = ['needs', 'look', 'byyou', 'sure'];

function handleOf(mark: Mark | PedicleMark): Handle {
  if (!('seen' in mark)) return { source: mark.source, pinned: mark.pinned };
  if (mark.seen === 'not-visible') return { source: mark.source, pinned: false };
  return { source: mark.source, pinned: mark.pinned };
}

function handlesOf(marks: VertebraMarks): Handle[] {
  switch (marks.kind) {
    case 'front':
    case 'side':
      return Object.values(marks.points).map(handleOf);
    case 'bending':
      return [...marks.lines.sup, ...marks.lines.inf].map(handleOf);
  }
}

/** True when the vertebra's level is one of its film's count's levels, from T1 down. */
function isCounted(film: FilmMarks, vertebra: VertebraOnFilm): boolean {
  return compareLevels(vertebra.level, 'T1') >= 0 && TEMPLATE_LEVELS[film.naming.template].includes(vertebra.level);
}

/** True when a person set the vertebra: a pinned point, a point they placed, an edit, or L5. */
function setByPerson(film: FilmMarks, vertebra: VertebraOnFilm): boolean {
  if (film.naming.l5ConfirmedOn === vertebra.id) return true;
  if (vertebra.confidence.reasons.includes('edited-by-user')) return true;
  return handlesOf(vertebra.marks).some((handle) => handle.pinned || PERSON_SOURCES.has(handle.source));
}

/** The state of one vertebra on one film. */
export function howSure(film: FilmMarks, vertebra: VertebraOnFilm): Conf {
  if (!isCounted(film, vertebra)) return 'sacrum';
  if (setByPerson(film, vertebra)) return 'byyou';
  return BAND_STATE[vertebra.confidence.band];
}

/**
 * A level's state across its films: the most urgent state any of them
 * shows, ignoring films where the level is not counted.
 */
export function levelState(states: readonly Conf[]): Conf {
  const counted: readonly Conf[] = states.filter((state) => state !== 'sacrum');
  if (counted.length === 0) return 'sacrum';
  return URGENCY.find((state) => counted.includes(state)) ?? 'sacrum';
}
