// The `confirm-l5` edit: the user says which vertebra is L5, and every
// vertebra on that film is named from it.

import { TEMPLATE_LEVELS, TEMPLATE_NAMES, type FilmMarks, type NamingFlag } from '@/shapes';

/**
 * The film's marks with `vertebra` named L5, the vertebrae above and
 * below it taking the names before and after L5 in its count's names,
 * `non-standard-count` flagged when a name falls outside the count's own
 * levels, and `l5ConfirmedOn` set to it. `null` when the film does not
 * hold that vertebra, or when some vertebra would run past the ends of
 * the names (the screens offer the edit only when every vertebra then
 * has a name).
 */
export function namedFromL5(marks: FilmMarks, vertebra: number): FilmMarks | null {
  const anchor = marks.vertebrae.findIndex((candidate) => candidate.id === vertebra);
  if (anchor === -1) return null;
  const template = marks.naming.template;
  const names = TEMPLATE_NAMES[template];
  const l5 = names.indexOf('L5');
  const vertebrae = [];
  for (const [index, named] of marks.vertebrae.entries()) {
    const level = names[l5 + index - anchor];
    if (level === undefined) return null;
    vertebrae.push(level === named.level ? named : { ...named, level });
  }
  const outside = vertebrae.some((named) => !TEMPLATE_LEVELS[template].includes(named.level));
  const flagged = marks.naming.flags.includes('non-standard-count');
  const flags: NamingFlag[] = outside && !flagged ? [...marks.naming.flags, 'non-standard-count'] : marks.naming.flags;
  return { ...marks, vertebrae, naming: { ...marks.naming, flags, l5ConfirmedOn: vertebra } };
}
