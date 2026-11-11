// The three standing checks the guide keeps in view, and where every other
// finding shows. Found, In order and Names are always there from the result
// on; extra rows appear only when a finding or a bending film needs them.

import { compareLevels } from '@/shapes';
import type { CheckFinding, CheckKind, FilmMarks, Level } from '@/shapes';
import { countedLevels } from './counted-levels';
import { countsDisagree, l5QuestionOpen } from './l5-question';
import { missingLevels } from './missing-levels';
import { levelCountsDiffer } from './same-count';

export type StandingRow =
  | { row: 'found'; state: 'ok' | 'wait' | 'bad'; found: number; of: number; missing: number }
  | { row: 'in-order'; state: 'ok' | 'bad'; reason: 'out-of-order' | 'films-disagree' | null }
  | { row: 'names'; state: 'warn' | 'ok'; reason: 'off-by-one' | 'names-unknown' | 'non-standard-count' | null }
  | { row: 'names'; state: 'you'; reason: 'non-standard-count' | null }
  | { row: 'other-facing'; state: 'warn' }
  | { row: 'bending-count'; state: 'warn'; film: 'bend-left' | 'bend-right' };

/** Where a finding is shown in Wave 1. */
export type Place =
  | 'found-row'
  | 'in-order-row'
  | 'names-row'
  | 'other-facing-row'
  | 'step-two-facts'
  | 'step-two-question'
  | 'bay-chip'
  | 'case-sheet'
  | 'on-film'
  | 'not-shown';

/** Whether the finding keeps "Everything checks out" from showing: yes, no, or until L5 is confirmed. */
export type Stops = 'yes' | 'no' | 'until-l5';

export interface Shown {
  place: Place;
  stops: Stops;
}

/** The rows for a case: the lead film is Front, or the first film when there is no Front. */
export function standingRows(films: readonly FilmMarks[], findings: readonly CheckFinding[]): StandingRow[] {
  const lead = films.find((film) => film.film === 'front') ?? films[0];
  if (!lead) return [];
  const kinds = new Set<CheckKind>(findings.map((finding) => finding.kind));
  const counted = countedLevels(lead);
  const missing = missingOnCase(films, lead, findings);
  const present = new Set<Level>(lead.vertebrae.map((vertebra) => vertebra.level));

  const foundState = kinds.has('vertebra-missing') ? 'bad' : l5QuestionOpen(lead) ? 'wait' : 'ok';
  const found: StandingRow = {
    row: 'found',
    state: foundState,
    found: counted.filter((level) => present.has(level) && !missing.has(level)).length,
    of: counted.length,
    missing: missing.size,
  };

  const outOfOrder = kinds.has('levels-out-of-order');
  const filmsDisagree = kinds.has('films-disagree');
  const inOrder: StandingRow = {
    row: 'in-order',
    state: outOfOrder || filmsDisagree ? 'bad' : 'ok',
    reason: outOfOrder ? 'out-of-order' : filmsDisagree ? 'films-disagree' : null,
  };

  const names = namesRow(films, lead, findings, kinds);

  const extras: StandingRow[] = [];
  if (kinds.has('marks-named-for-other-facing')) extras.push({ row: 'other-facing', state: 'warn' });
  for (const film of films) {
    if (film.film !== 'bend-left' && film.film !== 'bend-right') continue;
    if (levelCountsDiffer(lead, film)) extras.push({ row: 'bending-count', state: 'warn', film: film.film });
  }

  return [found, inOrder, names, ...extras];
}

/**
 * The levels from T1 to L5 of the lead film's count known to be missing:
 * those with no vertebra on the lead film or on any film a vertebra-missing
 * finding is about, and those such a finding names. A finding may name the
 * outline that swallowed the missing vertebra rather than an empty level,
 * so both are taken. The count is the lead film's, so a level it does not
 * count is left out.
 */
function missingOnCase(films: readonly FilmMarks[], lead: FilmMarks, findings: readonly CheckFinding[]): Set<Level> {
  const counted = countedLevels(lead).filter((level) => compareLevels(level, 'L5') <= 0);
  const missing = new Set<Level>(missingLevels(lead));
  for (const finding of findings) {
    if (finding.kind !== 'vertebra-missing') continue;
    const film = films.find((candidate) => candidate.film === finding.film) ?? lead;
    for (const level of missingLevels(film)) missing.add(level);
    if (finding.level !== null) missing.add(finding.level);
  }
  return new Set(counted.filter((level) => missing.has(level)));
}

/**
 * The names row. A names-unknown finding on any film, or a likely-off-by-one
 * on a film whose L5 nobody has confirmed, shows here whatever the lead
 * film's state, so no film's naming problem is hidden behind "set by you".
 */
function namesRow(films: readonly FilmMarks[], lead: FilmMarks, findings: readonly CheckFinding[], kinds: ReadonlySet<CheckKind>): StandingRow {
  const confirmed = (finding: CheckFinding): boolean =>
    (films.find((film) => film.film === finding.film) ?? lead).naming.l5ConfirmedOn !== null;
  const leadConfirmed = lead.naming.l5ConfirmedOn !== null;
  const offByOne = findings.some((finding) => finding.kind === 'likely-off-by-one' && !confirmed(finding));
  if (offByOne || (!leadConfirmed && countsDisagree(lead))) return { row: 'names', state: 'warn', reason: 'off-by-one' };
  if (kinds.has('names-unknown')) return { row: 'names', state: 'warn', reason: 'names-unknown' };
  if (leadConfirmed) return { row: 'names', state: 'you', reason: kinds.has('non-standard-count') ? 'non-standard-count' : null };
  if (kinds.has('non-standard-count')) return { row: 'names', state: 'warn', reason: 'non-standard-count' };
  return { row: 'names', state: 'ok', reason: null };
}

/** Where one finding shows, and whether it holds back "Everything checks out". */
export function whereFindingShows(finding: CheckFinding): Shown {
  switch (finding.kind) {
    case 'vertebra-missing':
      return { place: 'found-row', stops: 'yes' };
    case 'levels-out-of-order':
    case 'films-disagree':
      return { place: 'in-order-row', stops: 'yes' };
    case 'likely-off-by-one':
    case 'non-standard-count':
      return { place: 'names-row', stops: 'until-l5' };
    case 'names-unknown':
      return { place: 'names-row', stops: 'yes' };
    case 'l5-unconfirmed':
      return { place: 'not-shown', stops: 'no' };
    case 'upright-unconfirmed':
    case 'facing-unconfirmed':
    case 'scale-unconfirmed':
      return { place: 'step-two-facts', stops: 'no' };
    case 'left-right-unconfirmed':
      return finding.film === 'front'
        ? { place: 'step-two-question', stops: 'no' }
        : { place: 'bay-chip', stops: 'no' };
    case 'polarity-unconfirmed':
    case 'privacy-unchecked':
      return { place: 'case-sheet', stops: 'no' };
    case 'pedicle-not-visible':
      return { place: 'on-film', stops: 'no' };
    case 'marks-named-for-other-facing':
      return { place: 'other-facing-row', stops: 'yes' };
  }
}
