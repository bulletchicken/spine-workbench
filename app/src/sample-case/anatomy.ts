// The made-up spine every sample film is painted from: no patient and no
// image file, only these numbers. A port of the made-up spine the design
// mock paints, number for number, so that screens built on the sample
// case line up with the mock's screenshots.
//
// Points here are in the mock's canvas units: pixel (i, j) covers the
// square from (i, j) to (i + 1, j + 1). Film pixels put the centre of the
// top-left pixel at (0, 0), so `toFilmPixels` takes half a pixel off where
// a point leaves for the case (docs/conventions/pixel-space.md). Every
// view is described upright; the bending-left film is turned over only
// when it is stored (`films.ts`).

import type { FilmKind } from '@/shapes';

/** The mock's film size, in pixels. */
export const FILM_WIDTH = 720;
export const FILM_HEIGHT = 1720;

/**
 * Millimetres per film pixel. The mock used 0.6 and showed 0.144; at 0.45
 * the bodies come out at an adult's size (L5 about 49 mm wide) and the
 * 25 mm marker ball spans 55.6 pixels, enough for a two-click ruler.
 */
export const MM_PER_PX = 0.45;

/** The marker ball on the bending-right film, for the two-click ruler. */
export const BALL_DIAMETER_MM = 25;

/** The mock's bones, top to bottom: C7, the 17 from T1 to L5, then S1. */
export const BONE_NAMES = [
  'C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'S1',
] as const;

export interface Point {
  x: number;
  y: number;
}

export interface Pedicle extends Point {
  rx: number;
  ry: number;
  /** False for the one pedicle the film does not show (T1's on the patient's left). */
  seen: boolean;
}

export interface Bone {
  /** 0 for C7, 1 to 17 for T1 to L5, 18 for S1. */
  i: number;
  name: (typeof BONE_NAMES)[number];
  /** The body's centre. */
  c: Point;
  /** Unit vector down the spine's axis at the centre. */
  a: Point;
  /** Unit vector across the axis, toward image right. */
  p: Point;
  w: number;
  h: number;
  /** The top and bottom of the body along the image's vertical. */
  yt: number;
  yb: number;
  /** Corners: top left, top right, bottom right, bottom left, on the image. */
  q: [Point, Point, Point, Point];
  /** Front-view films only, image left then image right; none on C7 and S1. */
  ped: [Pedicle, Pedicle] | null;
}

/** One sample of the centre line, one per unit of arc length from its top. */
export interface CentreSample extends Point {
  /** Unit vector across the line, toward image right. */
  px: number;
  py: number;
  /** The axis's angle from image down, in radians. */
  ang: number;
}

export interface Truth {
  bones: Bone[];
  /** The centre line from 16 pixels above C7 to 16 below S1, by arc length. */
  byS: CentreSample[];
  len: number;
  /** The arc length at which the centre line reaches image row `y`. */
  sOfY: (y: number) => number;
  y0: number;
  y1: number;
}

export interface View {
  kind: FilmKind;
  /** Front and bending films are front views; the side film is the side view. */
  look: 'front' | 'side';
  /** How far the body leans sideways at image row `y`. */
  lean: (y: number) => number;
  /** The spine's centre at image row `y`. */
  X: (y: number) => number;
  /** The body's width at `k`, 0 at T1 and 1 at L5. */
  w: (k: number) => number;
  /** The seed of the film's own noise. */
  seed: number;
}

const clamp = (v: number, a = 0, b = 1) => Math.min(b, Math.max(a, v));
export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const gauss = (u: number) => Math.exp((-u * u) / 2);

const leanOf = (amount: number) => (y: number) => amount * Math.pow(clamp((1290 - y) / 1040), 1.7);
const frontWidth = (k: number) => lerp(58, 108, Math.pow(k, 1.05));

export const VIEWS: Record<FilmKind, View> = {
  front: {
    kind: 'front',
    look: 'front',
    lean: leanOf(0),
    X: (y) => 362 - 64 * gauss((y - 640) / 150) + 40 * gauss((y - 1095) / 118),
    w: frontWidth,
    seed: 11,
  },
  side: {
    kind: 'side',
    look: 'side',
    lean: leanOf(0),
    X: (y) => 318 - 56 * gauss((y - 600) / 215) + 50 * gauss((y - 1150) / 150),
    w: (k) => lerp(40, 70, k),
    seed: 23,
  },
  'bend-left': {
    kind: 'bend-left',
    look: 'front',
    lean: leanOf(150),
    X: (y) => 362 + leanOf(150)(y) - 70 * gauss((y - 640) / 150) + 12 * gauss((y - 1095) / 118),
    w: frontWidth,
    seed: 37,
  },
  'bend-right': {
    kind: 'bend-right',
    look: 'front',
    lean: leanOf(-150),
    X: (y) => 362 + leanOf(-150)(y) - 26 * gauss((y - 640) / 150) + 54 * gauss((y - 1095) / 118),
    w: frontWidth,
    seed: 53,
  },
};

/** The order the mock hangs the films in, which is also the case's. */
export const FILM_KINDS: readonly FilmKind[] = ['front', 'side', 'bend-left', 'bend-right'];

/** How far the pedicles sit off the body's middle, from the mock: the vertebra's turn. */
export const pedicleTurn = (y: number) => -gauss((y - 640) / 150) * 0.9 + gauss((y - 1095) / 118) * 0.5;

function buildTruth(view: View): Truth {
  const X = view.X;
  const dX = (y: number) => (X(y + 1) - X(y - 1)) / 2;
  const end = (y: number) => {
    const m = dX(y);
    const n = Math.hypot(m, 1);
    const a = { x: m / n, y: 1 / n };
    return { c: { x: X(y), y }, a, p: { x: a.y, y: -a.x } };
  };
  const bones: Bone[] = [];
  let y = 214;
  for (let i = 0; i < 19; i++) {
    const k = clamp((i - 1) / 16);
    const h = i === 0 ? 31 : i === 18 ? 56 : lerp(33, 61, Math.pow(k, 0.9));
    const disc = i === 0 ? 7 : lerp(7, 16, k);
    const w = i === 0 ? view.w(0) * 0.94 : i === 18 ? view.w(1) * 1.05 : view.w(k);
    const T = end(y);
    const B = end(y + h);
    const C = end(y + h / 2);
    const q: Bone['q'] = [
      { x: T.c.x - (T.p.x * w) / 2, y: T.c.y - (T.p.y * w) / 2 },
      { x: T.c.x + (T.p.x * w) / 2, y: T.c.y + (T.p.y * w) / 2 },
      { x: B.c.x + (B.p.x * w) / 2, y: B.c.y + (B.p.y * w) / 2 },
      { x: B.c.x - (B.p.x * w) / 2, y: B.c.y - (B.p.y * w) / 2 },
    ];
    let ped: Bone['ped'] = null;
    if (view.look === 'front' && i > 0 && i < 18) {
      const turn = pedicleTurn(C.c.y);
      const pedicle = (side: -1 | 1): Pedicle => ({
        x: C.c.x + C.p.x * (side * 0.3 + turn * 0.1) * w - C.a.x * h * 0.16,
        y: C.c.y + C.p.y * (side * 0.3 + turn * 0.1) * w - C.a.y * h * 0.16,
        rx: w * 0.105,
        ry: h * 0.17,
        seen: !(i === 1 && side === 1),
      });
      ped = [pedicle(-1), pedicle(1)];
    }
    bones.push({ i, name: BONE_NAMES[i] ?? 'S1', c: C.c, a: C.a, p: C.p, w, h, yt: y, yb: y + h, q, ped });
    y += h + disc;
  }

  const first = bones[0];
  const last = bones[18];
  if (first === undefined || last === undefined) throw new Error('the spine has 19 bones');
  const y0 = first.yt - 16;
  const y1 = last.yb + 16;
  const byS: CentreSample[] = [];
  let s = 0;
  let px = X(y0);
  let py = y0;
  for (let yy = y0; yy <= y1; yy += 0.5) {
    const x = X(yy);
    s += Math.hypot(x - px, yy - py);
    px = x;
    py = yy;
    while (byS.length <= s) {
      const e = end(yy);
      byS.push({ x: e.c.x, y: yy, px: e.p.x, py: e.p.y, ang: Math.atan2(e.a.x, e.a.y) });
    }
  }
  const sOfY = (yy: number) => {
    let lo = 0;
    let hi = byS.length - 1;
    while (lo < hi) {
      const m = (lo + hi) >> 1;
      if ((byS[m]?.y ?? Infinity) < yy) lo = m + 1;
      else hi = m;
    }
    return lo;
  };
  return { bones, byS, len: byS.length - 1, sOfY, y0, y1 };
}

const truths = new Map<FilmKind, Truth>();

/** The true spine on one film, upright, in the mock's canvas units. */
export function truthOf(kind: FilmKind): Truth {
  let truth = truths.get(kind);
  if (truth === undefined) {
    truth = buildTruth(VIEWS[kind]);
    truths.set(kind, truth);
  }
  return truth;
}

/** One bone of one film's truth. */
export function boneOf(kind: FilmKind, i: number): Bone {
  const bone = truthOf(kind).bones[i];
  if (bone === undefined) throw new Error(`there is no bone ${String(i)}`);
  return bone;
}

/** Where the lead marker's ring is centred, on the front-view films that show it. */
export function leadMarkerAt(view: View): Point {
  return { x: 360 + view.lean(150) + 238, y: 150 };
}

/** The marker ball on the bending-right film. */
export const BALL = { x: 176, y: 1150, r: BALL_DIAMETER_MM / MM_PER_PX / 2 };

/** The femoral heads the front-view films are painted with: image left, then image right. */
export const FRONT_HIPS = [
  { x: 360 - 176, y: 1502, r: 56 },
  { x: 360 + 176, y: 1502, r: 56 },
] as const;

/** The two femoral heads on the side film, painted a little apart. */
export function sideHips(): readonly [Point & { r: number }, Point & { r: number }] {
  const sx = boneOf('side', 18).c.x;
  return [
    { x: sx + 40, y: 1492, r: 54 },
    { x: sx + 58, y: 1486, r: 54 },
  ];
}

/**
 * A point of the upright picture in film pixels, as the film is stored:
 * half a pixel off, and the bending-left film turned over.
 */
export function toFilmPixels(kind: FilmKind, point: Point): [number, number] {
  if (kind === 'bend-left') return [FILM_WIDTH - point.x - 0.5, FILM_HEIGHT - point.y - 0.5];
  return [point.x - 0.5, point.y - 0.5];
}
