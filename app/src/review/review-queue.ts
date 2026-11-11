// What needs a person, in the order the review walks it: which vertebra is
// L5, then the missing vertebra, then each level that needs you, then each
// level worth a look, top to bottom within each group. The case in, the list
// out. Items the person skipped are left out.
//
// An item's key is what a skip remembers. A level's key follows the vertebra,
// not its name, because names move when L5 is set or an outline is split:
// `needs:<id>` or `look:<id>`, with the id of the lead film's vertebra at that
// level, or the least-sure film's when the lead film has none there.
//
// A vertebra one of the first two items is about is not asked about again
// as a level: the L5 question covers its two candidates while it is open,
// and the missing item covers the outline that swallowed the missing
// vertebra. Once the question is answered or the outline split, the queue is
// worked out afresh and anything still unsure comes back as a level.

import type { CheckFinding, FilmKind, FilmMarks, Level, Px, VertebraMarks, VertebraOnFilm } from '@/shapes';
import { disagreeingL5, l5QuestionOpen } from './l5-question';
import { levelStates } from './level-states';
import { missingLevels } from './missing-levels';
import { idsAcross, type FilmIds } from './same-level';

export type QueueItem =
  | { key: 'which-is-l5'; kind: 'which-is-l5'; level: 'L5'; film: FilmKind; a: FilmIds; b: FilmIds }
  | { key: 'missing'; kind: 'missing'; level: Level | null; film: FilmKind; ids: FilmIds }
  | { key: string; kind: 'needs' | 'look'; level: Level; film: FilmKind; ids: FilmIds };

/** The lead film is Front, or the first film when there is no Front. */
function leadOf(films: readonly FilmMarks[]): FilmMarks | undefined {
  return films.find((film) => film.film === 'front') ?? films[0];
}

/** The midpoints of a vertebra's upper and lower endplates, in film pixels. */
function endplateMidpoints(marks: VertebraMarks): { sup: Px; inf: Px } {
  const mid = (a: Px, b: Px): Px => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  switch (marks.kind) {
    case 'front':
      return {
        sup: mid(marks.points.supL.at, marks.points.supR.at),
        inf: mid(marks.points.infL.at, marks.points.infR.at),
      };
    case 'side':
      return {
        sup: mid(marks.points.supA.at, marks.points.supP.at),
        inf: mid(marks.points.infA.at, marks.points.infP.at),
      };
    case 'bending':
      return {
        sup: mid(marks.lines.sup[0].at, marks.lines.sup[1].at),
        inf: mid(marks.lines.inf[0].at, marks.lines.inf[1].at),
      };
  }
}

function heightPx(vertebra: VertebraOnFilm): number {
  const { sup, inf } = endplateMidpoints(vertebra.marks);
  return Math.hypot(inf[0] - sup[0], inf[1] - sup[1]);
}

/** The tallest vertebra on a film with the reason odd-height, or none. The first one wins a tie. */
function tallestOddHeight(film: FilmMarks): VertebraOnFilm | undefined {
  let tallest: VertebraOnFilm | undefined;
  let tallestPx = -Infinity;
  for (const vertebra of film.vertebrae) {
    if (!vertebra.confidence.reasons.includes('odd-height')) continue;
    const px = heightPx(vertebra);
    if (px > tallestPx) {
      tallest = vertebra;
      tallestPx = px;
    }
  }
  return tallest;
}

/** The review queue for a case, minus the items whose keys were skipped. */
export function reviewQueue(films: readonly FilmMarks[], findings: readonly CheckFinding[], skipped: readonly string[] = []): QueueItem[] {
  const lead = leadOf(films);
  if (!lead) return [];
  const items: QueueItem[] = [];
  const asked = new Set<Level>();

  const candidates = l5QuestionOpen(lead) ? disagreeingL5(lead) : null;
  if (candidates) {
    items.push({
      key: 'which-is-l5',
      kind: 'which-is-l5',
      level: 'L5',
      film: lead.film,
      a: idsAcross(films, lead, candidates.upper.id),
      b: idsAcross(films, lead, candidates.lower.id),
    });
    asked.add(candidates.upper.level).add(candidates.lower.level);
  }

  const missingFindings = findings.filter((finding) => finding.kind === 'vertebra-missing');
  const missing = missingFindings.find((finding) => finding.film === lead.film) ?? missingFindings[0];
  if (missing) {
    const onFilm = films.find((film) => film.film === missing.film) ?? lead;
    const target = tallestOddHeight(onFilm);
    items.push({
      key: 'missing',
      kind: 'missing',
      level: missing.level ?? missingLevels(onFilm)[0] ?? null,
      film: onFilm.film,
      ids: target ? idsAcross(films, onFilm, target.id) : {},
    });
    if (target) asked.add(target.level);
  }

  const needs: QueueItem[] = [];
  const look: QueueItem[] = [];
  for (const { level, state, onFilms } of levelStates(films)) {
    if ((state !== 'needs' && state !== 'look') || asked.has(level)) continue;
    const leastSure = onFilms.find((entry) => entry.state === state);
    if (!leastSure) continue;
    const from = films.find((film) => film.film === leastSure.film) ?? lead;
    const keyId = onFilms.find((entry) => entry.film === lead.film)?.id ?? leastSure.id;
    const item: QueueItem = { key: `${state}:${String(keyId)}`, kind: state, level, film: leastSure.film, ids: idsAcross(films, from, leastSure.id) };
    (state === 'needs' ? needs : look).push(item);
  }

  return [...items, ...needs, ...look].filter((item) => !skipped.includes(item.key));
}
