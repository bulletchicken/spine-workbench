// One film's saved marks, turned into what the overlay draws. Which look
// each vertebra has, its spoken line, and whether it is shown yet are
// worked out by the caller (the review rule, the words and the replay's
// clock), so this file knows only shapes.

import { compareLevels, type FilmMarks, type VertebraMarks, type VertebraOnFilm } from '@/shapes';
import type { OutlineLook } from '@/look-and-feel/data-attributes';
import type { MarkToDraw, Quad } from './mark-to-draw';

/** The corners of one vertebra, going round its outline. */
function quadOf(marks: VertebraMarks): Quad {
  switch (marks.kind) {
    case 'front': {
      const { supL, supR, infL, infR } = marks.points;
      return [supL.at, supR.at, infR.at, infL.at];
    }
    case 'side': {
      const { supA, supP, infA, infP } = marks.points;
      return [supA.at, supP.at, infP.at, infA.at];
    }
    case 'bending': {
      // The two endplate lines, joined into one closed outline.
      const { sup, inf } = marks.lines;
      return [sup[0].at, sup[1].at, inf[1].at, inf[0].at];
    }
  }
}

/**
 * Every outline the overlay draws for one film, top to bottom. Anything
 * above T1 is left out: C7 is not outlined. `read` gives each vertebra's
 * look and spoken line; `shown` says whether it is drawn yet.
 */
export function fromFilmMarks(
  film: FilmMarks,
  read: (vertebra: VertebraOnFilm) => { look: OutlineLook; spoken: string },
  shown: (vertebra: VertebraOnFilm) => boolean,
): MarkToDraw[] {
  return film.vertebrae
    .filter((vertebra) => compareLevels(vertebra.level, 'T1') >= 0)
    .map((vertebra) => {
      const { look, spoken } = read(vertebra);
      return { key: String(vertebra.id), name: vertebra.level, quad: quadOf(vertebra.marks), look, shown: shown(vertebra), spoken };
    });
}
