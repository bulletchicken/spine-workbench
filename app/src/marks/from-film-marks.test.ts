// The sample case's marks, turned into what the overlay draws. The story
// is the sample case's own: 18 bodies on Front and Side, C7 the first of
// them, endplate lines on the bending films.

import { describe, expect, it } from 'vitest';
import type { FilmKind, FilmMarks, VertebraOnFilm } from '@/shapes';
import { caseAt } from '@/sample-case/stages';
import { fromFilmMarks } from './from-film-marks';

const marksOf = (kind: FilmKind): FilmMarks => {
  const film = caseAt('pass-result').case.marks.find((marks) => marks.film === kind);
  if (film === undefined) throw new Error(`the sample case has no marks on ${kind}`);
  return film;
};

const read = (vertebra: VertebraOnFilm) => ({ look: vertebra.level === 'T3' ? ('look' as const) : ('found' as const), spoken: `${vertebra.level}, said` });
const always = () => true;

/** The vertebra of that level on the film. */
function at(film: FilmMarks, level: string): VertebraOnFilm {
  const found = film.vertebrae.find((vertebra) => vertebra.level === level);
  if (found === undefined) throw new Error(`no ${level}`);
  return found;
}

describe('fromFilmMarks', () => {
  it('draws each vertebra from T1 down under its id, with its name and what the caller says of it', () => {
    const film = marksOf('front');
    const marks = fromFilmMarks(film, read, always);
    expect(film.vertebrae[0]?.level).toBe('C7');
    expect(marks).toHaveLength(17);
    expect(marks.map((mark) => mark.key)).toEqual(film.vertebrae.slice(1).map((vertebra) => String(vertebra.id)));
    expect(marks.map((mark) => mark.name)).not.toContain('C7');
    expect(marks[0]?.name).toBe('T1');
    expect(marks[2]).toMatchObject({ name: 'T3', look: 'look', spoken: 'T3, said' });
    expect(marks[3]?.look).toBe('found');
  });

  it('takes a front film’s corners in the order that goes round the outline', () => {
    const film = marksOf('front');
    const t5 = at(film, 'T5');
    if (t5.marks.kind !== 'front') throw new Error('the front film holds front marks');
    const { supL, supR, infL, infR } = t5.marks.points;
    expect(fromFilmMarks(film, read, always).find((mark) => mark.name === 'T5')?.quad).toEqual([supL.at, supR.at, infR.at, infL.at]);
  });

  it('takes a side film’s corners front, back, back, front', () => {
    const film = marksOf('side');
    const t5 = at(film, 'T5');
    if (t5.marks.kind !== 'side') throw new Error('the side film holds side marks');
    const { supA, supP, infA, infP } = t5.marks.points;
    expect(fromFilmMarks(film, read, always).find((mark) => mark.name === 'T5')?.quad).toEqual([supA.at, supP.at, infP.at, infA.at]);
  });

  it('joins a bending film’s two endplate lines into one closed outline', () => {
    const film = marksOf('bend-left');
    const t5 = at(film, 'T5');
    if (t5.marks.kind !== 'bending') throw new Error('a bending film holds endplate lines');
    const { sup, inf } = t5.marks.lines;
    expect(fromFilmMarks(film, read, always).find((mark) => mark.name === 'T5')?.quad).toEqual([sup[0].at, sup[1].at, inf[1].at, inf[0].at]);
  });

  it('passes on whether each one is shown yet', () => {
    const film = marksOf('front');
    const marks = fromFilmMarks(film, read, (vertebra) => vertebra.level === 'T2' || vertebra.level === 'L5');
    expect(marks.filter((mark) => mark.shown).map((mark) => mark.name)).toEqual(['T2', 'L5']);
  });
});
