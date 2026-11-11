// Front and the film beside it show the same level. Films differ in size,
// scale and turn, so the link is by level, not by pixels: the level under
// the source's look spot, as a fraction of the way between two vertebrae
// both films have, is put under the partner's look spot at the same
// multiple of its fit. With no shared levels, the same fraction of the
// film's upright height stands in for the level.

import type { FilmMarks, Level, Px, VertebraOnFilm } from '@/shapes';
import { apply, filmSpace, invert, uprightMatrix, uprightSize } from '../film-space';
import type { FilmPlacement, FilmSpace } from '../film-space';
import { fit, isFit, lookPoint, travelTo, zoomOverFit, type CameraFrame } from './camera';

export interface SourceFilm {
  frame: CameraFrame;
  placement: FilmPlacement;
  marks: FilmMarks | null;
}

export interface PartnerFilm {
  frame: CameraFrame;
  marks: FilmMarks | null;
}

/** The partner's placement that shows the level the source is looking at. */
export function followPartner(source: SourceFilm, partner: PartnerFilm): FilmPlacement {
  if (isFit(source.frame, source.placement)) return fit(partner.frame);
  const point = partnerPoint(source, partner);
  return travelTo(partner.frame, point, zoomOverFit(source.frame, source.placement));
}

/** The middle of a vertebra's marks: its four corners, or its two endplate lines' ends. */
export function vertebraCentre(vertebra: VertebraOnFilm): Px {
  const marks = vertebra.marks;
  const corners: Px[] =
    marks.kind === 'front'
      ? [marks.points.supL.at, marks.points.supR.at, marks.points.infL.at, marks.points.infR.at]
      : marks.kind === 'side'
        ? [marks.points.supA.at, marks.points.supP.at, marks.points.infA.at, marks.points.infP.at]
        : [marks.lines.sup[0].at, marks.lines.sup[1].at, marks.lines.inf[0].at, marks.lines.inf[1].at];
  return [corners.reduce((sum, p) => sum + p[0], 0) / 4, corners.reduce((sum, p) => sum + p[1], 0) / 4];
}

/** One level both films have, as upright points on each. */
interface Shared {
  source: Px;
  partner: Px;
}

function partnerPoint(source: SourceFilm, partner: PartnerFilm): Px {
  const toSourceUpright = uprightMatrix(source.frame.size, source.frame.view);
  const toPartnerUpright = uprightMatrix(partner.frame.size, partner.frame.view);
  const look = lookPoint(source.frame, source.placement);
  const pairs = sharedLevels(source, partner, toSourceUpright, toPartnerUpright);
  if (pairs.length === 0) return byHeight(source, partner, look);
  return apply(invert(toPartnerUpright), byLevel(pairs, apply(toSourceUpright, look)[1], source, partner));
}

function sharedLevels(
  source: SourceFilm,
  partner: PartnerFilm,
  toSourceUpright: ReturnType<typeof uprightMatrix>,
  toPartnerUpright: ReturnType<typeof uprightMatrix>,
): Shared[] {
  if (source.marks === null || partner.marks === null) return [];
  const onPartner = new Map<Level, Px>();
  for (const vertebra of partner.marks.vertebrae) onPartner.set(vertebra.level, apply(toPartnerUpright, vertebraCentre(vertebra)));
  const pairs: Shared[] = [];
  for (const vertebra of source.marks.vertebrae) {
    const there = onPartner.get(vertebra.level);
    if (there !== undefined) pairs.push({ source: apply(toSourceUpright, vertebraCentre(vertebra)), partner: there });
  }
  return pairs.sort((a, b) => a.source[1] - b.source[1]);
}

/** Between two shared levels: the same fraction of the way. Beyond the ends: the end level, offset at the films' relative scale. */
function byLevel(pairs: Shared[], y: number, source: SourceFilm, partner: PartnerFilm): Px {
  const first = pairs[0];
  const last = pairs[pairs.length - 1];
  if (first === undefined || last === undefined) throw new Error('byLevel needs at least one shared level');
  if (y <= first.source[1] || pairs.length === 1) return offsetFrom(first, y, scaleBetween(pairs, source, partner));
  if (y >= last.source[1]) return offsetFrom(last, y, scaleBetween(pairs, source, partner));
  for (let i = 1; i < pairs.length; i += 1) {
    const above = pairs[i - 1];
    const below = pairs[i];
    if (above === undefined || below === undefined || y > below.source[1]) continue;
    const gap = below.source[1] - above.source[1];
    const t = gap === 0 ? 0 : (y - above.source[1]) / gap;
    return [above.partner[0] + t * (below.partner[0] - above.partner[0]), above.partner[1] + t * (below.partner[1] - above.partner[1])];
  }
  return offsetFrom(last, y, scaleBetween(pairs, source, partner));
}

/** Partner pixels per source pixel, down the spine: from the shared levels' span, or else from the films' heights. */
function scaleBetween(pairs: Shared[], source: SourceFilm, partner: PartnerFilm): number {
  const first = pairs[0];
  const last = pairs[pairs.length - 1];
  if (first !== undefined && last !== undefined && last.source[1] > first.source[1]) {
    return (last.partner[1] - first.partner[1]) / (last.source[1] - first.source[1]);
  }
  return uprightHeight(partner.frame) / uprightHeight(source.frame);
}

function offsetFrom(level: Shared, y: number, scale: number): Px {
  return [level.partner[0], level.partner[1] + (y - level.source[1]) * scale];
}

/** No shared levels: the same fraction of each film's upright width and height, from the source's film point to the partner's. */
function byHeight(source: SourceFilm, partner: PartnerFilm, look: Px): Px {
  const from = uprightSize(source.frame.size, source.frame.view.quarterTurns);
  const to = uprightSize(partner.frame.size, partner.frame.view.quarterTurns);
  // Measured from the outer corner, so the film's edges match exactly.
  const across = outerCorner(source.frame).toScreen(look);
  return outerCorner(partner.frame).toFilm([(across[0] / from.width) * to.width, (across[1] / from.height) * to.height]);
}

/** The upright film one screen pixel to a film pixel, its outer corner at the origin: film points as distances from that corner. */
function outerCorner(frame: CameraFrame): FilmSpace {
  return filmSpace({ size: frame.size, view: frame.view, zoom: 1, pan: [0, 0] });
}

function uprightHeight(frame: CameraFrame): number {
  return uprightSize(frame.size, frame.view.quarterTurns).height;
}
