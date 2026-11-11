// The edits that change one point on one film without the engine:
// `move-mark`, `pin-mark` and `hide-pedicle`. Each returns the film's new
// marks, or `null` when the point it names is not there (the vertebra is
// gone, the mark belongs to another view, the landmark was never found,
// or a pedicle that cannot be seen is to be pinned).

import type {
  FilmMarks,
  HipCentre,
  LandmarkName,
  Landmarks,
  Mark,
  MarkName,
  PedicleMark,
  PinTarget,
  Px,
  VertebraMarks,
  VertebraOnFilm,
} from '@/shapes';

/** What happens to the one point: a placed mark and a pedicle each change their own way. */
interface PointChange {
  mark(mark: Mark): Mark;
  pedicle(pedicle: PedicleMark): PedicleMark | null;
  hip(hip: HipCentre): HipCentre;
}

/**
 * Put the point exactly at `to`: its source becomes `hand`, its pinned
 * flag stays, and it is no longer inferred. A pedicle that could not be
 * seen is placed, unpinned; a pedicle or a hip centre placed by hand
 * loses its ring score or fitted radius, which described where the
 * engine found it.
 *
 * `to` is kept as given, wherever it is. The strict reader refuses a
 * point that is not finite or lies more than a film width or height off
 * its film (`is_near_film` in engine/shapes/src/pixels.rs), so the tool
 * that makes the move keeps `to` on the film; a drag that leaves the
 * film must not carry the point with it.
 */
export function movePoint(marks: FilmMarks, target: PinTarget, to: Px): FilmMarks | null {
  const at: Px = [to[0], to[1]];
  const moved = (mark: Mark): Mark => ({ at, source: 'hand', pinned: mark.pinned, inferred: false });
  return changePoint(marks, target, {
    mark: moved,
    pedicle: (pedicle) => ({
      seen: 'visible',
      at,
      source: 'hand',
      pinned: pedicle.seen === 'visible' && pedicle.pinned,
      inferred: false,
      ringScore: null,
    }),
    hip: (hip) => ({ ...hip, centre: moved(hip.centre), radiusPx: null }),
  });
}

/** Pin or unpin the point where it is. A pedicle that cannot be seen has no point to pin. */
export function pinPoint(marks: FilmMarks, target: PinTarget, pinned: boolean): FilmMarks | null {
  const pin = (mark: Mark): Mark => ({ ...mark, pinned });
  return changePoint(marks, target, {
    mark: pin,
    pedicle: (pedicle) => (pedicle.seen === 'visible' ? { ...pedicle, pinned } : null),
    hip: (hip) => ({ ...hip, centre: pin(hip.centre) }),
  });
}

/** The user says a front film's pedicle cannot be seen. */
export function hidePedicle(marks: FilmMarks, vertebra: number, mark: 'pedL' | 'pedR'): FilmMarks | null {
  return changePoint(
    marks,
    { on: 'vertebra', vertebra, mark },
    {
      mark: (placed) => placed,
      pedicle: () => ({ seen: 'not-visible', source: 'hand' }),
      hip: (hip) => hip,
    },
  );
}

function changePoint(marks: FilmMarks, target: PinTarget, change: PointChange): FilmMarks | null {
  if (target.on === 'landmark') {
    const landmarks = changeLandmark(marks.landmarks, target.landmark, change);
    return landmarks === null ? null : { ...marks, landmarks };
  }
  const index = marks.vertebrae.findIndex((vertebra) => vertebra.id === target.vertebra);
  const vertebra = marks.vertebrae[index];
  if (vertebra === undefined) return null;
  const changed = target.on === 'vertebra' ? changeNamed(vertebra.marks, target.mark, change) : changeLineEnd(vertebra.marks, target, change);
  if (changed === null) return null;
  const vertebrae: VertebraOnFilm[] = marks.vertebrae.with(index, { ...vertebra, marks: changed });
  return { ...marks, vertebrae };
}

function changeNamed(marks: VertebraMarks, name: MarkName, change: PointChange): VertebraMarks | null {
  if (marks.kind === 'front') {
    const points = marks.points;
    switch (name) {
      case 'supL':
      case 'supR':
      case 'infL':
      case 'infR':
        return { kind: 'front', points: { ...points, [name]: change.mark(points[name]) } };
      case 'pedL':
      case 'pedR': {
        const pedicle = change.pedicle(points[name]);
        return pedicle === null ? null : { kind: 'front', points: { ...points, [name]: pedicle } };
      }
      default:
        return null;
    }
  }
  if (marks.kind === 'side') {
    const points = marks.points;
    switch (name) {
      case 'supA':
      case 'supP':
      case 'infA':
      case 'infP':
        return { kind: 'side', points: { ...points, [name]: change.mark(points[name]) } };
      default:
        return null;
    }
  }
  return null;
}

function changeLineEnd(marks: VertebraMarks, target: Extract<PinTarget, { on: 'endplate' }>, change: PointChange): VertebraMarks | null {
  if (marks.kind !== 'bending') return null;
  const line = marks.lines[target.line];
  const end = target.end === 'patient-left' ? 0 : 1;
  const ends: [Mark, Mark] = end === 0 ? [change.mark(line[0]), line[1]] : [line[0], change.mark(line[1])];
  return { kind: 'bending', lines: { ...marks.lines, [target.line]: ends } };
}

function changeLandmark(landmarks: Landmarks, name: LandmarkName, change: PointChange): Landmarks | null {
  const top = landmarks.sacrumTop;
  switch (name) {
    case 'sacL':
    case 'sacR':
      if (top?.view !== 'front') return null;
      return { ...landmarks, sacrumTop: { ...top, [name]: change.mark(top[name]) } };
    case 'sacA':
    case 'sacP':
      if (top?.view !== 'side') return null;
      return { ...landmarks, sacrumTop: { ...top, [name]: change.mark(top[name]) } };
    case 'hipLeft':
      return changeHip(landmarks, landmarks.hips.findIndex((hip) => hip.side === 'left'), change);
    case 'hipRight':
      return changeHip(landmarks, landmarks.hips.findIndex((hip) => hip.side === 'right'), change);
    case 'hipFirst':
    case 'hipSecond': {
      const sideFilm = landmarks.hips.flatMap((hip, index) => (hip.side === 'unpaired' || hip.side === 'merged' ? [index] : []));
      return changeHip(landmarks, sideFilm[name === 'hipFirst' ? 0 : 1] ?? -1, change);
    }
  }
}

function changeHip(landmarks: Landmarks, index: number, change: PointChange): Landmarks | null {
  const hip = landmarks.hips[index];
  if (hip === undefined) return null;
  return { ...landmarks, hips: landmarks.hips.with(index, change.hip(hip)) };
}
