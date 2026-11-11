// The stand-in's case checks, worked out from the case they are given, so
// the review and the checks card can be seen to change as the case does.
// They follow the sample case's `findingsAt` rule for rule, so a screen in
// development and a gallery of the same stage show the same findings, and
// add the checks the sample story never trips. Film by film, in the case's
// order:
//
// - `vertebra-missing` (blocking): for each outline still carrying
//   `odd-height`, which stands in for the vertebra it swallowed, pointing
//   at it; and on the front and side films, for each level from T1 to L5
//   the film's count expects that no vertebra carries.
// - `levels-out-of-order` (blocking): levels that do not run strictly
//   down the list of levels.
// - `likely-off-by-one` (warning): the two counts disagree
//   (`ends-disagree`) while L5 is not confirmed, pointing at the vertebra
//   named L5.
// - `non-standard-count` (warning): the film's naming says so, pointing
//   at the first vertebra outside its count.
// - `names-unknown` (blocking): the film's naming says so.
// - `l5-unconfirmed` (warning): marks with no L5 confirmed.
// - `marks-named-for-other-facing` (blocking): marks named for a facing
//   the film's `imageRight` does not state, because it is still unknown or
//   because it differs.
// - The unconfirmed facts: upright; left and right on front and bending
//   films (blocking on the front film, whose pedicles it names); facing on
//   the side film; scale, brightness and the privacy check (warnings).
// - `pedicle-not-visible` (warning): each front pedicle marked not
//   visible, pointing at its vertebra.
//
// Then, for the case: `films-disagree` (blocking) when front and side name
// different levels.

import type { CaseFileV1, CheckFinding, CheckKind, FilmKind, FilmMarks, Level, Severity } from '@/shapes';
import { LEVELS, TEMPLATE_LEVELS } from '@/shapes';

export function checkCase(c: CaseFileV1): CheckFinding[] {
  const findings: CheckFinding[] = [];
  const find = (kind: CheckKind, severity: Severity, film: FilmKind | null, vertebra?: { id: number | null; level: Level } | null) => {
    findings.push({ kind, severity, film, vertebraId: vertebra?.id ?? null, level: vertebra?.level ?? null });
  };

  const kinds = [...c.films.map((record) => record.kind), ...c.marks.map((marks) => marks.film)].filter((kind, at, all) => all.indexOf(kind) === at);
  for (const film of kinds) {
    const marks = c.marks.find((each) => each.film === film);
    const geometry = c.films.find((record) => record.kind === film)?.geometry;

    if (marks !== undefined) {
      const { vertebrae, naming } = marks;
      for (const vertebra of vertebrae) {
        if (vertebra.confidence.reasons.includes('odd-height')) find('vertebra-missing', 'blocking', film, vertebra);
      }
      if (film === 'front' || film === 'side') {
        for (const level of TEMPLATE_LEVELS[naming.template].filter(fromT1ToL5)) {
          if (!vertebrae.some((vertebra) => vertebra.level === level)) find('vertebra-missing', 'blocking', film, { id: null, level });
        }
      }
      const order = vertebrae.map((vertebra) => LEVELS.indexOf(vertebra.level));
      if (order.some((at, index) => index > 0 && at <= (order[index - 1] ?? -1))) find('levels-out-of-order', 'blocking', film);
      if (naming.l5ConfirmedOn === null && naming.flags.includes('ends-disagree')) {
        find('likely-off-by-one', 'warning', film, vertebrae.find((vertebra) => vertebra.level === 'L5'));
      }
      if (naming.flags.includes('non-standard-count')) {
        const counted = TEMPLATE_LEVELS[naming.template];
        find('non-standard-count', 'warning', film, vertebrae.find((vertebra) => !counted.includes(vertebra.level)));
      }
      if (naming.flags.includes('names-unknown')) find('names-unknown', 'blocking', film);
      if (naming.l5ConfirmedOn === null) find('l5-unconfirmed', 'warning', film);
      if (geometry !== undefined && geometry.imageRight !== marks.namedFor) find('marks-named-for-other-facing', 'blocking', film);
    }

    if (geometry !== undefined) {
      const { confirmed } = geometry;
      if (!confirmed.upright) find('upright-unconfirmed', 'warning', film);
      if (film !== 'side' && !confirmed.leftRight) find('left-right-unconfirmed', film === 'front' ? 'blocking' : 'warning', film);
      if (film === 'side' && !confirmed.facing) find('facing-unconfirmed', 'warning', film);
      if (!confirmed.scale) find('scale-unconfirmed', 'warning', film);
      if (!confirmed.polarity) find('polarity-unconfirmed', 'warning', film);
      if (!confirmed.privacy) find('privacy-unchecked', 'warning', film);
    }

    for (const vertebra of marks?.vertebrae ?? []) {
      if (vertebra.marks.kind !== 'front') continue;
      for (const pedicle of [vertebra.marks.points.pedL, vertebra.marks.points.pedR]) {
        if (pedicle.seen === 'not-visible') find('pedicle-not-visible', 'warning', film, vertebra);
      }
    }
  }

  const front = c.marks.find((marks) => marks.film === 'front');
  const side = c.marks.find((marks) => marks.film === 'side');
  if (front !== undefined && side !== undefined && !sameLevels(front, side)) find('films-disagree', 'blocking', null);
  return findings;
}

function fromT1ToL5(level: Level): boolean {
  const at = LEVELS.indexOf(level);
  return at >= LEVELS.indexOf('T1') && at <= LEVELS.indexOf('L5');
}

function sameLevels(a: FilmMarks, b: FilmMarks): boolean {
  const levels = (marks: FilmMarks) => marks.vertebrae.map((vertebra) => vertebra.level).join(' ');
  return levels(a) === levels(b);
}
