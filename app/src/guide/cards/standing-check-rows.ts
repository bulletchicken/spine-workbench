// The standing checks as rows the checks card shows: each of the review's
// rows worded once, here, so the result of the pass and the review that
// follows it read the same.

import type { StandingRow } from '@/review/standing-checks';
import type { FilmKind } from '@/shapes';
import { STANDING_WORDS } from '@/words/checks';
import type { CardRow } from './CardRows';

/**
 * The card rows for the review's standing rows, in the order given.
 * `filmName` is how the bench names a film, for a row about a bending
 * film.
 */
export function standingCheckRows(rows: readonly StandingRow[], filmName: (film: FilmKind) => string): CardRow[] {
  return rows.map((row): CardRow => {
    switch (row.row) {
      case 'found':
        if (row.state === 'wait') return said('found', row.state, STANDING_WORDS.foundOutlines(row.found), STANDING_WORDS.foundWaitLine);
        return said('found', row.state, STANDING_WORDS.foundOf(row.found, row.of), row.missing > 0 ? STANDING_WORDS.foundMissingLine(row.missing) : null);
      case 'in-order':
        if (row.reason === 'out-of-order') return said('in-order', row.state, STANDING_WORDS.inOrderOutOfOrder, null);
        if (row.reason === 'films-disagree') return said('in-order', row.state, STANDING_WORDS.inOrderFilmsDisagree, null);
        return said('in-order', row.state, STANDING_WORDS.inOrder, null);
      case 'names':
        return namesRow(row);
      case 'other-facing':
        return said('other-facing', row.state, STANDING_WORDS.otherFacing, null);
      case 'bending-count':
        return said(`bending-count:${row.film}`, row.state, STANDING_WORDS.bendingCount(filmName(row.film)), null);
    }
  });
}

function namesRow(row: Extract<StandingRow, { row: 'names' }>): CardRow {
  switch (row.reason) {
    case 'off-by-one':
      return said('names', row.state, STANDING_WORDS.namesOffByOne, STANDING_WORDS.namesOffByOneLine);
    case 'names-unknown':
      return said('names', row.state, STANDING_WORDS.namesUnknown, STANDING_WORDS.namesUnknownLine);
    case 'non-standard-count':
      // Once L5 is confirmed the names are set, and the unusual count is said under them.
      return row.state === 'you'
        ? said('names', row.state, STANDING_WORDS.namesSet, STANDING_WORDS.namesNonStandard)
        : said('names', row.state, STANDING_WORDS.namesNonStandard, STANDING_WORDS.namesNonStandardLine);
    case null:
      return said('names', row.state, STANDING_WORDS.namesSet, row.state === 'you' ? STANDING_WORDS.namesSetLine : null);
  }
}

function said(id: string, state: CardRow['state'], title: string, line: string | null): CardRow {
  return line === null ? { id, state, title } : { id, state, title, lines: [line] };
}
