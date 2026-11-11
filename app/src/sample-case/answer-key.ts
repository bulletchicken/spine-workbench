// The sample case's answer key: the truth its films were painted from, in
// the shape a made-up spine hands over, so "Try a made-up spine" on the
// stand-in brings the sample case with everything true about it.
//
// The truth is the anatomy itself, not the story's marks: T11 and T12 are
// two vertebrae, T3's outline reaches its bone, and the outline on the
// sacrum is no vertebra. The standing spine's pose is put together from
// the front and side films (patient frame: x forward, y to the patient's
// left, z up, in mm, from the centre of the underside of L5). Its turn
// about its own axis is read from how far the painted pedicles sit off
// the body's middle: a body turned by ψ moves pedicles half a body's
// depth behind its centre sideways by that half depth times sin ψ. The
// angles are measured from these poses and the true endplates, never
// taken from anywhere else.
//
// The sample case is not drawn from a recipe. Its key records the
// anatomy's own seed (the front film's, 11) under `random`, at tier t2:
// noise, a vignette and markers, nothing missing and nothing in the way.

import type {
  AnswerKey,
  CurveAngle,
  CurveRegion,
  FilmAnswer,
  FilmGeometry,
  FilmKind,
  Level,
  TrueLandmarks,
  TrueMark,
  TrueVertebra,
  VertebraPose,
} from '@/shapes';
import { TEMPLATE_LEVELS } from '@/shapes';
import { boneOf, FILM_KINDS, FRONT_HIPS, MM_PER_PX, pedicleTurn, sideHips, VIEWS, type Bone, type Point } from './anatomy';
import { defaultPedicles, filmPoint } from './marks';
import { rulerAcrossTheBall } from './stages';

const LEVELS: readonly Level[] = TEMPLATE_LEVELS.standard;
const BONES = Array.from({ length: LEVELS.length }, (_, i) => i);

const seen = (kind: FilmKind, point: Point, visible = true): TrueMark => ({ at: filmPoint(kind, point), visible });

/** The true marks of one bone on one film, named for the patient's left on the image right and anterior on the right. */
function trueVertebra(kind: FilmKind, bone: Bone): TrueVertebra {
  const level = LEVELS[bone.i] ?? 'L5';
  const [tl, tr, br, bl] = bone.q;
  if (kind === 'side') {
    return { level, marks: { kind: 'side', supA: seen(kind, tr), supP: seen(kind, tl), infA: seen(kind, br), infP: seen(kind, bl) } };
  }
  if (kind !== 'front') {
    return { level, marks: { kind: 'bending', sup: [seen(kind, tr), seen(kind, tl)], inf: [seen(kind, br), seen(kind, bl)] } };
  }
  let pedL: TrueMark;
  let pedR: TrueMark;
  if (bone.ped === null) {
    // C7's pedicles are not painted: where they usually sit, and not seen.
    const at = defaultPedicles(filmPoint(kind, tr), filmPoint(kind, tl), filmPoint(kind, br), filmPoint(kind, bl));
    pedL = { at: at.pedL, visible: false };
    pedR = { at: at.pedR, visible: false };
  } else {
    pedL = seen(kind, bone.ped[1], bone.ped[1].seen);
    pedR = seen(kind, bone.ped[0], bone.ped[0].seen);
  }
  return { level, marks: { kind: 'front', supL: seen(kind, tr), supR: seen(kind, tl), infL: seen(kind, br), infR: seen(kind, bl), pedL, pedR } };
}

function trueLandmarks(kind: FilmKind): TrueLandmarks {
  const c7 = boneOf(kind, 0);
  const corners = [c7.q[1], c7.q[0], c7.q[3], c7.q[2]].map((point) => filmPoint(kind, point));
  const mean = (axis: 0 | 1) => corners.reduce((sum, at) => sum + at[axis], 0) / 4;
  const sacrum = boneOf(kind, 18);
  const middle = { x: (sacrum.q[0].x + sacrum.q[1].x) / 2, y: (sacrum.q[0].y + sacrum.q[1].y) / 2 };
  const [imageLeft, imageRight] = kind === 'side' ? sideHips() : FRONT_HIPS;
  return {
    c7: { at: [mean(0), mean(1)], visible: true },
    sacrumTop: [seen(kind, sacrum.q[1]), seen(kind, middle), seen(kind, sacrum.q[0])],
    // On the side film the two heads are painted without saying which is which: the one nearer the back first.
    hips: kind === 'side' ? [seen(kind, imageLeft), seen(kind, imageRight)] : [seen(kind, imageRight), seen(kind, imageLeft)],
  };
}

/** Rows of a 3 × 3 matrix. */
type Matrix = [[number, number, number], [number, number, number], [number, number, number]];

const multiply = (a: Matrix, b: Matrix): Matrix =>
  [0, 1, 2].map((r) => [0, 1, 2].map((c) => [0, 1, 2].reduce((sum, k) => sum + (a[r]?.[k] ?? 0) * (b[k]?.[c] ?? 0), 0))) as Matrix;

const rotX = (t: number): Matrix => [[1, 0, 0], [0, Math.cos(t), -Math.sin(t)], [0, Math.sin(t), Math.cos(t)]];
const rotY = (t: number): Matrix => [[Math.cos(t), 0, Math.sin(t)], [0, 1, 0], [-Math.sin(t), 0, Math.cos(t)]];
const rotZ = (t: number): Matrix => [[Math.cos(t), -Math.sin(t), 0], [Math.sin(t), Math.cos(t), 0], [0, 0, 1]];

/** The rotation vector of a rotation matrix: its axis times its angle. */
export function rotationVector(m: Matrix): [number, number, number] {
  const trace = m[0][0] + m[1][1] + m[2][2];
  const angle = Math.acos(Math.min(1, Math.max(-1, (trace - 1) / 2)));
  const axis: [number, number, number] = [m[2][1] - m[1][2], m[0][2] - m[2][0], m[1][0] - m[0][1]];
  const sine = Math.sin(angle);
  if (sine < 1e-12) return [axis[0] / 2, axis[1] / 2, axis[2] / 2];
  return [(axis[0] * angle) / (2 * sine), (axis[1] * angle) / (2 * sine), (axis[2] * angle) / (2 * sine)];
}

/** The angle of a line on the film from +x toward +y, in radians. */
const angleOf = (from: Point, to: Point) => Math.atan2(to.y - from.y, to.x - from.x);

/** How far a body turns about its own axis, read from its painted pedicles; positive turns the front to the patient's left. */
function twistOf(front: Bone, side: Bone): number {
  const shift = pedicleTurn(front.c.y) * 0.1 * front.w;
  return -Math.asin(Math.max(-1, Math.min(1, shift / (side.w / 2))));
}

/** One bone's pose from a front-view film and the side film. */
function poseOf(frontKind: FilmKind, i: number): VertebraPose {
  const front = boneOf(frontKind, i);
  const side = boneOf('side', i);
  const originFront = boneOf(frontKind, 17);
  const originSide = boneOf('side', 17);
  const bottomFront = VIEWS[frontKind].X(originFront.yb);
  const bottomSide = VIEWS.side.X(originSide.yb);
  const dXf = front.a.x / front.a.y;
  const dXs = side.a.x / side.a.y;
  const length = Math.hypot(dXs, dXf, 1);
  const tilt = Math.atan(dXf);
  const lean = Math.asin(-dXs / length);
  const rotation = multiply(multiply(rotX(tilt), rotY(lean)), rotZ(twistOf(front, side)));
  const top = { f: VIEWS[frontKind].X(front.yt), s: VIEWS.side.X(side.yt) };
  const bottom = { f: VIEWS[frontKind].X(front.yb), s: VIEWS.side.X(side.yb) };
  return {
    level: LEVELS[i] ?? 'L5',
    centre: [(side.c.x - bottomSide) * MM_PER_PX, (front.c.x - bottomFront) * MM_PER_PX, (originFront.yb - front.c.y) * MM_PER_PX],
    rotation: rotationVector(rotation),
    height: Math.hypot(bottom.s - top.s, bottom.f - top.f, front.h) * MM_PER_PX,
    // The front view puts the patient's left on the image right; the side view puts the front there.
    wedgeFront: angleOf(front.q[3], front.q[2]) - angleOf(front.q[0], front.q[1]),
    wedgeSide: angleOf(side.q[0], side.q[1]) - angleOf(side.q[3], side.q[2]),
    topHalfWidth: (front.w / 2) * MM_PER_PX,
    topHalfDepth: (side.w / 2) * MM_PER_PX,
    bottomHalfWidth: (front.w / 2) * MM_PER_PX,
    bottomHalfDepth: (side.w / 2) * MM_PER_PX,
  };
}

const NOTHING_LEFT_OPEN = { upright: true, polarity: true, privacy: true, scale: true } as const;

/** Each film's true geometry, as if every fact had been confirmed. */
function trueGeometry(kind: FilmKind): FilmGeometry {
  const fromFile = { mmPerPx: MM_PER_PX, mmPerPxVertical: null, scale: { source: 'file', rule: 'geometry' } } as const;
  switch (kind) {
    case 'front':
      return { ...fromFile, quarterTurns: 0, imageRight: 'patient-left', confirmed: { ...NOTHING_LEFT_OPEN, leftRight: true, facing: false } };
    case 'side':
      return { ...fromFile, quarterTurns: 0, imageRight: 'anterior', confirmed: { ...NOTHING_LEFT_OPEN, leftRight: false, facing: true } };
    case 'bend-left':
      return { ...fromFile, quarterTurns: 2, imageRight: 'patient-left', confirmed: { ...NOTHING_LEFT_OPEN, leftRight: true, facing: false } };
    case 'bend-right': {
      const ruler = rulerAcrossTheBall();
      return {
        mmPerPx: ruler.mm / Math.hypot(ruler.b[0] - ruler.a[0], ruler.b[1] - ruler.a[1]),
        mmPerPxVertical: null,
        scale: { source: 'ruler', rulers: [ruler] },
        quarterTurns: 0,
        imageRight: 'patient-left',
        confirmed: { ...NOTHING_LEFT_OPEN, leftRight: true, facing: false },
      };
    }
  }
}

function filmAnswer(kind: FilmKind): FilmAnswer {
  return {
    kind,
    geometry: trueGeometry(kind),
    vertebrae: BONES.map((i) => trueVertebra(kind, boneOf(kind, i))),
    landmarks: trueLandmarks(kind),
    bentPoses: kind === 'bend-left' || kind === 'bend-right' ? BONES.map((i) => poseOf(kind, i)) : null,
  };
}

const degrees = (radians: number) => Math.round(((radians * 180) / Math.PI) * 100) / 100;

/** The regions curves are measured in, by the bones they span. */
const REGIONS: readonly [CurveRegion, number, number][] = [
  ['proximal-thoracic', 1, 6],
  ['main-thoracic', 5, 12],
  ['thoracolumbar-lumbar', 11, 17],
];

/** The largest angle between an upper endplate and a lower one at or below it, within the bones. */
function cobbOf(kind: FilmKind, first: number, last: number): number {
  let largest = 0;
  for (let i = first; i <= last; i++) {
    const upper = boneOf(kind, i);
    for (let j = i; j <= last; j++) {
      const lower = boneOf(kind, j);
      largest = Math.max(largest, Math.abs(angleOf(upper.q[0], upper.q[1]) - angleOf(lower.q[3], lower.q[2])));
    }
  }
  return largest;
}

/** The bone furthest from the straight line between a region's ends, on the front film. */
function apexOf(first: number, last: number): number {
  const a = boneOf('front', first).c;
  const b = boneOf('front', last).c;
  let apex = first;
  let furthest = -1;
  for (let i = first; i <= last; i++) {
    const c = boneOf('front', i).c;
    const away = Math.abs((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)) / Math.hypot(b.x - a.x, b.y - a.y);
    if (away > furthest) {
      furthest = away;
      apex = i;
    }
  }
  return apex;
}

export function answerKey(): AnswerKey {
  const frontalCobb: CurveAngle[] = REGIONS.map(([region, first, last]) => ({ region, degrees: degrees(cobbOf('front', first, last)) }));
  const apicalTwist: CurveAngle[] = REGIONS.map(([region, first, last]) => {
    const apex = apexOf(first, last);
    return { region, degrees: degrees(twistOf(boneOf('front', apex), boneOf('side', apex))) };
  });
  const t4 = boneOf('side', 4);
  const t12 = boneOf('side', 12);
  const l1 = boneOf('side', 13);
  const s1 = boneOf('side', 18);
  return {
    recipe: { name: 'random', seed: VIEWS.front.seed, tier: 't2' },
    template: 'standard',
    poses: BONES.map((i) => poseOf('front', i)),
    films: FILM_KINDS.map(filmAnswer),
    angles: {
      frontalCobb,
      apicalTwist,
      kyphosisDegrees: degrees(Math.abs(angleOf(t4.q[0], t4.q[1]) - angleOf(t12.q[3], t12.q[2]))),
      lordosisDegrees: degrees(Math.abs(angleOf(l1.q[0], l1.q[1]) - angleOf(s1.q[0], s1.q[1]))),
    },
  };
}
