// Marks against a made-up spine's answer key, as plain geometry: each true
// vertebra is matched to the marked vertebra whose corners' centre is
// nearest its own, within half its height; its level is right when the
// names agree, and its corner error is the distance from each true corner
// to the marked corner of the same name.

import type { AnswerKey, AnswerKeyComparison, FilmComparison, FilmMarks, Px, TrueMarks, VertebraComparison } from '@/shapes';
import { axisOf, centroid, distance, quadOf, type Quad } from './quads';

export function compareWithAnswerKey(marks: FilmMarks[], key: AnswerKey): AnswerKeyComparison {
  return { films: key.films.map((answer) => compareFilm(answer, marks.find((film) => film.film === answer.kind))) };
}

function compareFilm(answer: AnswerKey['films'][number], marks: FilmMarks | undefined): FilmComparison {
  const found = marks?.vertebrae ?? [];
  const vertebrae: VertebraComparison[] = answer.vertebrae.map((truth) => {
    const trueQuad = trueQuadOf(truth.marks);
    const centre = centroid(trueQuad);
    const reach = axisOf(trueQuad).height / 2;
    const nearest = found
      .map((vertebra) => ({ vertebra, gap: distance(centroid(quadOf(vertebra)), centre) }))
      .filter((candidate) => candidate.gap <= reach)
      .sort((a, b) => a.gap - b.gap)[0]?.vertebra;
    if (nearest === undefined) return { level: truth.level, vertebraId: null, levelRight: false, meanCornerErrorPx: null, maxCornerErrorPx: null };
    const errors = quadOf(nearest).map((corner, slot) => distance(corner, trueQuad[slot] as Px));
    return {
      level: truth.level,
      vertebraId: nearest.id,
      levelRight: nearest.level === truth.level,
      meanCornerErrorPx: mean(errors),
      maxCornerErrorPx: Math.max(...errors),
    };
  });
  const matched = vertebrae.flatMap((vertebra) => (vertebra.meanCornerErrorPx === null ? [] : [vertebra.meanCornerErrorPx]));
  const meanCornerErrorPx = matched.length === 0 ? null : mean(matched);
  const mmPerPx = answer.geometry.mmPerPx;
  return {
    film: answer.kind,
    expected: answer.vertebrae.length,
    found: found.length,
    levelsRight: vertebrae.filter((vertebra) => vertebra.levelRight).length,
    meanCornerErrorPx,
    meanCornerErrorMm: meanCornerErrorPx === null || mmPerPx === null ? null : meanCornerErrorPx * mmPerPx,
    vertebrae,
  };
}

function trueQuadOf(marks: TrueMarks): Quad {
  switch (marks.kind) {
    case 'front':
      return [marks.supL.at, marks.supR.at, marks.infR.at, marks.infL.at];
    case 'side':
      return [marks.supA.at, marks.supP.at, marks.infP.at, marks.infA.at];
    case 'bending':
      return [marks.sup[0].at, marks.sup[1].at, marks.inf[1].at, marks.inf[0].at];
  }
}

function mean(values: number[]): number {
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}
