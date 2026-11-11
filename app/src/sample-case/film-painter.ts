// Paints the four sample films from the anatomy: a port of the way the
// design mock paints its stand-in X-rays, drawing call for drawing call,
// on plain arrays instead of a canvas so it runs the same in a test and
// in the browser. Dense things are bright.
//
// The mock paints two white layers, soft tissue and bone, blurs them onto
// black, adds the lead marker and the marker ball, then grain and a
// vignette. The result here is one 8-bit grey per pixel, upright.

import type { FilmKind } from '@/shapes';
import {
  BALL,
  FILM_HEIGHT,
  FILM_WIDTH,
  leadMarkerAt,
  lerp,
  truthOf,
  VIEWS,
  type Bone,
  type Point,
  type View,
} from './anatomy';
import { gaussianBlur } from './painting/blur';
import { fillCoverage, strokeCoverage, type Coverage } from './painting/coverage';
import { Path } from './painting/path';
import { ellipsePath, WhiteLayer } from './painting/white-layer';

export interface PaintedFilm {
  /** The picture before grain and vignette, which the evidence maps are worked out from. */
  clean: Uint8Array;
  /** The film itself. */
  film: Uint8Array;
}

/** The mock's seeded random numbers (mulberry32), so the grain and the haze match it. */
export function seededRandom(seed: number): () => number {
  let state = seed;
  return () => {
    state |= 0;
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const clamp = (v: number, a = 0, b = 1) => Math.min(b, Math.max(a, v));

/** A body's outline: a box with its corners rounded and its sides pulled in, as the mock draws it. */
function bodyPath(c: WhiteLayer, q: Bone['q']): void {
  const [a, b, d, e] = q;
  const mid = (p: Point, r: Point, t: number) => ({ x: lerp(p.x, r.x, t), y: lerp(p.y, r.y, t) });
  const centre = { x: (a.x + b.x + d.x + e.x) / 4, y: (a.y + b.y + d.y + e.y) / 4 };
  const pull = (p: Point, f: number) => ({ x: lerp(p.x, centre.x, f), y: lerp(p.y, centre.y, f) });
  const r = 0.17;
  const sides: [Point, Point, number][] = [
    [a, b, 0.08],
    [b, d, 0.2],
    [d, e, 0.08],
    [e, a, 0.2],
  ];
  c.beginPath();
  const start = mid(a, b, r);
  c.moveTo(start.x, start.y);
  sides.forEach(([p, n, f], k) => {
    const m = pull(mid(p, n, 0.5), f);
    const sideEnd = mid(p, n, 1 - r);
    c.quadraticCurveTo(m.x, m.y, sideEnd.x, sideEnd.y);
    const [np, nn] = sides[(k + 1) % 4] ?? sides[0] ?? [a, b, 0];
    const nextStart = mid(np, nn, r);
    c.quadraticCurveTo(n.x, n.y, nextStart.x, nextStart.y);
  });
  c.closePath();
}

function paintSpine(n: WhiteLayer, view: View): void {
  const bones = truthOf(view.kind).bones;
  const front = view.look === 'front';
  const c7 = bones[0];
  if (c7 === undefined) return;
  // A few neck bones above C7, so the top of the film reads as a neck.
  for (let j = 1; j <= 4; j++) {
    const y = c7.yt - 8 - j * 31;
    const x = view.X(y);
    const w = c7.w * (1 - j * 0.04);
    n.fillAlpha = 0.11;
    n.beginPath();
    n.roundRect(x - w / 2, y, w, 25, 6);
    n.fill();
    n.strokeAlpha = 0.16;
    n.lineWidth = 2;
    n.stroke();
  }
  for (const bone of bones) {
    const { c, a, p, w, h, q } = bone;
    const thoracic = bone.i <= 12;
    if (front) {
      // The transverse processes either side.
      for (const side of [-1, 1]) {
        const L = thoracic ? 12 : 30;
        const x = c.x + p.x * side * (w / 2 + L * 0.7) - a.x * h * 0.18;
        const y = c.y + p.y * side * (w / 2 + L * 0.7) - a.y * h * 0.18;
        n.fillAlpha = 0.085;
        ellipsePath(n, x, y, L, thoracic ? 8 : 9, Math.atan2(p.y, p.x));
        n.fill();
      }
    } else {
      // The parts behind the body, and the spinous process pointing down and back.
      n.fillAlpha = 0.085;
      ellipsePath(n, c.x - p.x * (w / 2 + 15), c.y - p.y * (w / 2 + 15) - h * 0.08, 17, h * 0.4);
      n.fill();
      const L = thoracic ? 60 : 58;
      const drop = thoracic ? h * 0.95 : h * 0.2;
      n.strokeAlpha = 0.085;
      n.lineWidth = thoracic ? 13 : 25;
      n.beginPath();
      n.moveTo(c.x - p.x * (w / 2 + 22), c.y - p.y * (w / 2 + 22));
      n.lineTo(c.x - p.x * (w / 2 + 22 + L), c.y - p.y * (w / 2 + 22 + L) + drop);
      n.stroke();
    }
    bodyPath(n, q);
    n.fillAlpha = front ? 0.15 : 0.19;
    n.fill();
    n.strokeAlpha = 0.27;
    n.lineWidth = 2.6;
    n.stroke();
    // The two endplates, brighter.
    n.strokeAlpha = 0.2;
    n.lineWidth = 3.6;
    n.beginPath();
    const along = (P: Point, Q: Point, t: number) => ({ x: lerp(P.x, Q.x, t), y: lerp(P.y, Q.y, t) });
    let u = along(q[0], q[1], 0.1);
    let v = along(q[0], q[1], 0.9);
    n.moveTo(u.x, u.y + 1.5);
    n.lineTo(v.x, v.y + 1.5);
    u = along(q[3], q[2], 0.1);
    v = along(q[3], q[2], 0.9);
    n.moveTo(u.x, u.y - 1.5);
    n.lineTo(v.x, v.y - 1.5);
    n.stroke();
    if (front) {
      // The spinous process seen end on, and the pedicle rings.
      n.fillAlpha = 0.08;
      ellipsePath(n, c.x + a.x * h * 0.1, c.y + a.y * h * 0.1, w * 0.065, h * 0.36, -Math.atan2(a.x, a.y));
      n.fill();
      for (const pedicle of bone.ped ?? []) {
        const strength = pedicle.seen ? 1 : 0.28;
        ellipsePath(n, pedicle.x, pedicle.y, pedicle.rx, pedicle.ry, -Math.atan2(a.x, a.y));
        n.fillAlpha = 0.09 * strength;
        n.fill();
        n.strokeAlpha = 0.3 * strength;
        n.lineWidth = 2.6;
        n.stroke();
      }
    }
  }
}

function paintFrontBody(t: WhiteLayer, n: WhiteLayer, view: View): void {
  const lean = view.lean;
  const cx = (y: number) => 360 + lean(y);
  const random = seededRandom(view.seed);
  const profile: [number, number][] = [
    [-10, 96], [110, 100], [190, 150], [246, 304], [330, 322], [520, 296],
    [800, 268], [1010, 242], [1150, 266], [1320, 312], [1500, 326], [1730, 320],
  ];
  t.fillAlpha = 0.6;
  t.beginPath();
  profile.forEach(([y, hw], i) => {
    if (i === 0) t.moveTo(cx(y) - hw, y);
    else t.lineTo(cx(y) - hw, y);
  });
  for (let i = profile.length - 1; i >= 0; i--) {
    const [y, hw] = profile[i] ?? [0, 0];
    t.lineTo(cx(y) + hw, y);
  }
  t.fill();
  t.fillAlpha = 0.45;
  ellipsePath(t, cx(0), -24, 116, 150);
  t.fill();
  t.fillAlpha = 0.3; // the shoulder girdle's haze
  ellipsePath(t, cx(300), 300, 330, 88);
  t.fill();
  t.composite = 'destination-out'; // the lungs are air: dark
  t.fillAlpha = 0.74;
  ellipsePath(t, cx(520) - 124, 532, 98, 250, 0.05);
  t.fill();
  ellipsePath(t, cx(520) + 128, 520, 96, 240, -0.05);
  t.fill();
  t.fillAlpha = 0.22;
  for (let i = 0; i < 9; i++) {
    const x = cx(1000) + (random() - 0.5) * 330;
    const y = 900 + random() * 250;
    const rx = 24 + random() * 40;
    const ry = 14 + random() * 22;
    const rotation = random() * 3;
    ellipsePath(t, x, y, rx, ry, rotation);
    t.fill();
  }
  t.composite = 'source-over';
  t.fillAlpha = 0.3; // the heart
  ellipsePath(t, cx(690) + 36, 690, 104, 120, 0.2);
  t.fill();
  t.strokeAlpha = 0.2;
  t.lineWidth = 112;
  t.beginPath();
  for (let y = 250; y <= 830; y += 20) {
    if (y === 250) t.moveTo(view.X(y), y);
    else t.lineTo(view.X(y), y);
  }
  t.stroke();
  t.fillAlpha = 0.12;
  ellipsePath(t, cx(900) - 120, 905, 125, 80);
  t.fill();
  // The base of the skull and the jaw.
  n.fillAlpha = 0.16;
  ellipsePath(n, cx(0), -40, 112, 150);
  n.fill();
  n.strokeAlpha = 0.17;
  n.lineWidth = 20;
  n.beginPath();
  n.ellipse(cx(50), 52, 86, 72, 0, 0.12 * Math.PI, 0.88 * Math.PI);
  n.stroke();
  // The ribs and their cartilage.
  const bones = truthOf(view.kind).bones;
  const chest = [0, 150, 188, 214, 232, 244, 252, 256, 256, 250, 236, 196, 140];
  const chestX = (y: number) => 360 + lean(y) + (view.X(y) - lean(y) - 360) * 0.3;
  for (let i = 1; i <= 12; i++) {
    const b = bones[i];
    const reach = chest[i] ?? 0;
    if (b === undefined) continue;
    for (const side of [-1, 1]) {
      const s0 = {
        x: b.c.x + b.p.x * side * (b.w / 2 - 3) - b.a.x * b.h * 0.28,
        y: b.c.y + b.p.y * side * (b.w / 2 - 3) - b.a.y * b.h * 0.28,
      };
      const drop = 22 + i * 6.5;
      const L = { x: chestX(s0.y + drop) + side * reach, y: s0.y + drop };
      n.beginPath();
      n.moveTo(s0.x, s0.y);
      n.bezierCurveTo(s0.x + side * reach * 0.42, s0.y - 30 - i, L.x - side * 2, L.y - 64, L.x, L.y);
      n.strokeAlpha = 0.085;
      n.lineWidth = 12 + i * 0.25;
      n.stroke();
      n.strokeAlpha = 0.06;
      n.lineWidth = 3;
      n.stroke();
      if (i <= 10) {
        const E = { x: chestX(L.y + 110) + side * (34 + i * 9), y: L.y + 84 + i * 11 };
        n.beginPath();
        n.moveTo(L.x, L.y);
        n.bezierCurveTo(L.x + side * 3, L.y + 62 + i * 3, E.x + side * reach * 0.42, E.y - 10, E.x, E.y);
        n.strokeAlpha = 0.045;
        n.lineWidth = 13;
        n.stroke();
      }
    }
  }
  // Collarbones, shoulder blades, the heads of the arms.
  for (const side of [-1, 1]) {
    const c0 = cx(262);
    n.beginPath();
    n.moveTo(c0 + side * 24, 268);
    n.bezierCurveTo(c0 + side * 110, 292, c0 + side * 190, 236, c0 + side * 278, 240);
    n.strokeAlpha = 0.17;
    n.lineWidth = 17;
    n.stroke();
    n.fillAlpha = 0.06;
    n.beginPath();
    n.moveTo(c0 + side * 150, 252);
    n.lineTo(c0 + side * 284, 262);
    n.lineTo(c0 + side * 252, 520);
    n.lineTo(c0 + side * 172, 430);
    n.closePath();
    n.fill();
    n.fillAlpha = 0.14;
    ellipsePath(n, c0 + side * 304, 296, 46, 46);
    n.fill();
    n.strokeAlpha = 0.11;
    n.lineWidth = 44;
    n.beginPath();
    n.moveTo(c0 + side * 318, 330);
    n.lineTo(cx(700) + side * 352, 720);
    n.stroke();
  }
  // The pelvis, the hips and the tops of the thighs.
  const pc = 360;
  for (const side of [-1, 1]) {
    n.beginPath();
    n.moveTo(pc + side * 50, 1262);
    n.bezierCurveTo(pc + side * 96, 1182, pc + side * 190, 1130, pc + side * 262, 1178);
    n.bezierCurveTo(pc + side * 300, 1210, pc + side * 296, 1300, pc + side * 262, 1392);
    n.bezierCurveTo(pc + side * 250, 1430, pc + side * 220, 1456, pc + side * 186, 1452);
    n.bezierCurveTo(pc + side * 150, 1440, pc + side * 120, 1420, pc + side * 72, 1410);
    n.bezierCurveTo(pc + side * 60, 1360, pc + side * 56, 1300, pc + side * 50, 1262);
    n.fillAlpha = 0.13;
    n.fill();
    n.strokeAlpha = 0.17;
    n.lineWidth = 4;
    n.stroke();
    n.fillAlpha = 0.15;
    ellipsePath(n, pc + side * 176, 1502, 56, 56);
    n.fill();
    n.strokeAlpha = 0.14;
    n.lineWidth = 3;
    n.stroke();
    n.strokeAlpha = 0.12;
    n.lineWidth = 60;
    n.beginPath();
    n.moveTo(pc + side * 190, 1520);
    n.lineTo(pc + side * 262, 1572);
    n.lineTo(pc + side * 250, 1740);
    n.stroke();
    n.fillAlpha = 0.08;
    ellipsePath(n, pc + side * 278, 1556, 30, 40);
    n.fill();
    n.strokeAlpha = 0.11;
    n.lineWidth = 24;
    ellipsePath(n, pc + side * 88, 1594, 58, 70);
    n.stroke();
  }
  // The sacrum, with its holes.
  n.beginPath();
  n.moveTo(pc - 80, 1268);
  n.lineTo(pc + 84, 1268);
  n.bezierCurveTo(pc + 62, 1384, pc + 28, 1452, pc + 2, 1478);
  n.bezierCurveTo(pc - 26, 1452, pc - 60, 1384, pc - 80, 1268);
  n.fillAlpha = 0.13;
  n.fill();
  n.strokeAlpha = 0.12;
  n.lineWidth = 3;
  n.stroke();
  n.composite = 'destination-out';
  n.fillAlpha = 0.5;
  for (let j = 0; j < 4; j++) {
    for (const side of [-1, 1]) {
      ellipsePath(n, pc + side * (46 - j * 8), 1338 + j * 34, 9, 8);
      n.fill();
    }
  }
  n.composite = 'lighter';
}

function paintSideBody(t: WhiteLayer, n: WhiteLayer, view: View): void {
  const back: [number, number][] = [
    [-10, 236], [150, 214], [250, 170], [330, 128], [600, 112], [900, 150],
    [1100, 190], [1240, 150], [1380, 112], [1500, 120], [1730, 170],
  ];
  const belly: [number, number][] = [
    [-10, 470], [140, 452], [230, 430], [300, 500], [520, 588], [800, 604],
    [1050, 590], [1250, 566], [1400, 548], [1730, 500],
  ];
  t.fillAlpha = 0.6;
  t.beginPath();
  back.forEach(([y, x], i) => {
    if (i === 0) t.moveTo(x, y);
    else t.lineTo(x, y);
  });
  for (let i = belly.length - 1; i >= 0; i--) {
    const [y, x] = belly[i] ?? [0, 0];
    t.lineTo(x, y);
  }
  t.fill();
  t.fillAlpha = 0.45;
  ellipsePath(t, 400, -30, 150, 160);
  t.fill();
  t.fillAlpha = 0.3;
  ellipsePath(t, 380, 300, 230, 90);
  t.fill();
  t.composite = 'destination-out';
  t.fillAlpha = 0.62;
  ellipsePath(t, 430, 525, 128, 235, 0.06);
  t.fill();
  t.composite = 'source-over';
  t.fillAlpha = 0.28;
  ellipsePath(t, 476, 664, 88, 100);
  t.fill();
  n.fillAlpha = 0.15;
  ellipsePath(n, 410, -36, 140, 150);
  n.fill();
  n.strokeAlpha = 0.1;
  n.lineWidth = 44;
  n.beginPath();
  n.moveTo(352, 302);
  n.lineTo(592, 172);
  n.moveTo(362, 318);
  n.lineTo(604, 204);
  n.stroke();
  n.strokeAlpha = 0.06;
  n.lineWidth = 16;
  n.beginPath();
  n.moveTo(560, 332);
  n.lineTo(598, 592);
  n.stroke();
  const bones = truthOf(view.kind).bones;
  for (let i = 1; i <= 12; i++) {
    const b = bones[i];
    if (b === undefined) continue;
    for (const [ox, oy] of [
      [0, 0],
      [-14, 7],
    ] as const) {
      const x = b.c.x - 4 + ox;
      const y = b.c.y + oy;
      const ex = Math.min(x + 150 + i * 10, x + 262);
      const ey = y + 110 + i * 10;
      n.beginPath();
      n.moveTo(x, y);
      n.bezierCurveTo(x + 60, y + 10, ex - 30, ey - 60, ex, ey);
      n.strokeAlpha = 0.045;
      n.lineWidth = 11;
      n.stroke();
    }
  }
  const s = bones[18];
  if (s === undefined) return;
  const sx = s.c.x;
  n.fillAlpha = 0.12;
  n.beginPath();
  n.moveTo(s.q[3].x, s.q[3].y);
  n.lineTo(s.q[2].x, s.q[2].y);
  n.lineTo(sx - 18, 1424);
  n.lineTo(sx - 72, 1506);
  n.lineTo(sx - 100, 1492);
  n.lineTo(sx - 64, 1400);
  n.closePath();
  n.fill();
  n.fillAlpha = 0.075;
  ellipsePath(n, sx - 6, 1336, 134, 120, -0.3);
  n.fill();
  n.fillAlpha = 0.12;
  ellipsePath(n, sx + 40, 1492, 54, 54);
  n.fill();
  ellipsePath(n, sx + 58, 1486, 54, 54);
  n.fill();
  n.strokeAlpha = 0.12;
  n.lineWidth = 64;
  n.beginPath();
  n.moveTo(sx + 62, 1522);
  n.lineTo(sx + 84, 1740);
  n.stroke();
}

/** Paint `alpha` of white through `cover` onto the opaque picture, in grey `shade`. */
function paintOnto(picture: Float32Array, { x0, y0, width, height, cover }: Coverage, alpha: number, shade = 1): void {
  for (let row = 0; row < height; row++) {
    const from = (y0 + row) * FILM_WIDTH + x0;
    for (let col = 0; col < width; col++) {
      const a = (cover[row * width + col] ?? 0) * alpha;
      if (a === 0) continue;
      const was = picture[from + col] ?? 0;
      picture[from + col] = was + a * (shade - was);
    }
  }
}

/** Lay a blurred white layer over the picture; a white `screen` comes to the same. */
function layOver(picture: Float32Array, layer: WhiteLayer, sigma: number, alpha: number): void {
  const blurred = gaussianBlur(layer.white, FILM_WIDTH, FILM_HEIGHT, sigma);
  for (let k = 0; k < picture.length; k++) {
    const a = (blurred[k] ?? 0) * alpha;
    const was = picture[k] ?? 0;
    picture[k] = was + a * (1 - was);
  }
}

/** The lead letter L a radiographer tapes to the cassette, and its ring. */
function paintLeadMarker(picture: Float32Array, at: Point): void {
  const turn = -0.06;
  const place = (x: number, y: number) => ({
    x: at.x + x * Math.cos(turn) - y * Math.sin(turn),
    y: at.y + x * Math.sin(turn) + y * Math.cos(turn),
  });
  // A bold sans-serif L, 36 pixels high, centred on the ring as the mock's
  // 50-pixel type sets it.
  const letter = new Path();
  const outline: [number, number][] = [
    [-13, -18], [-3, -18], [-3, 10], [15, 10], [15, 18], [-13, 18],
  ];
  outline.forEach(([x, y], k) => {
    const p = place(x, y);
    if (k === 0) letter.moveTo(p.x, p.y);
    else letter.lineTo(p.x, p.y);
  });
  letter.closePath();
  paintOnto(picture, fillCoverage(letter, FILM_WIDTH, FILM_HEIGHT), 0.9);
  const ring = new Path();
  ring.arc(at.x, at.y, 36, 0, 7);
  paintOnto(picture, strokeCoverage(ring, 3, FILM_WIDTH, FILM_HEIGHT), 0.55);
}

/**
 * The 25 mm marker ball, shaded as the mock's radial gradient: from a
 * small circle up and to the left of the centre (white at 0.98) out to
 * the rim (a slightly darker grey at 0.9).
 */
function paintBall(picture: Float32Array): void {
  const disc = new Path();
  disc.arc(BALL.x, BALL.y, BALL.r, 0, Math.PI * 2);
  const { x0, y0, width, height, cover } = fillCoverage(disc, FILM_WIDTH, FILM_HEIGHT);
  const inner = { x: BALL.x - 5, y: BALL.y - 6, r: 2 };
  const outer = { x: BALL.x, y: BALL.y, r: BALL.r };
  const dcx = outer.x - inner.x;
  const dcy = outer.y - inner.y;
  const dr = outer.r - inner.r;
  for (let row = 0; row < height; row++) {
    for (let col = 0; col < width; col++) {
      const c = cover[row * width + col] ?? 0;
      if (c === 0) continue;
      // The largest t with the point on the circle between the two:
      // |p − c(t)| = r(t), a quadratic a·t² − 2b·t + c = 0.
      const px = x0 + col + 0.5 - inner.x;
      const py = y0 + row + 0.5 - inner.y;
      const qa = dcx * dcx + dcy * dcy - dr * dr;
      const qb = px * dcx + py * dcy + inner.r * dr;
      const qc = px * px + py * py - inner.r * inner.r;
      const root = Math.sqrt(Math.max(0, qb * qb - qa * qc));
      const t = clamp(Math.max((qb + root) / qa, (qb - root) / qa));
      const alpha = lerp(0.98, 0.9, t);
      const shade = lerp(1, 225 / 255, t);
      const k = (y0 + row) * FILM_WIDTH + x0 + col;
      const was = picture[k] ?? 0;
      picture[k] = was + c * alpha * (shade - was);
    }
  }
}

function paint(kind: FilmKind): PaintedFilm {
  const view = VIEWS[kind];
  const picture = new Float32Array(FILM_WIDTH * FILM_HEIGHT);
  const tissue = new WhiteLayer(FILM_WIDTH, FILM_HEIGHT);
  const bone = new WhiteLayer(FILM_WIDTH, FILM_HEIGHT);
  bone.composite = 'lighter';
  if (view.look === 'front') paintFrontBody(tissue, bone, view);
  else paintSideBody(tissue, bone, view);
  paintSpine(bone, view);
  layOver(picture, tissue, 17, 0.37);
  layOver(picture, bone, 9, 0.3);
  layOver(picture, bone, 1.1, 0.92);
  if (view.look === 'front') paintLeadMarker(picture, leadMarkerAt(view));
  if (kind === 'bend-right') paintBall(picture);

  const clean = new Uint8Array(picture.length);
  for (let k = 0; k < picture.length; k++) clean[k] = Math.round(clamp(picture[k] ?? 0) * 255);

  // Grain and the falling-off of light toward the edges.
  const film = new Uint8Array(picture.length);
  const random = seededRandom(view.seed * 7 + 1);
  for (let y = 0, k = 0; y < FILM_HEIGHT; y++) {
    const vy = (y / FILM_HEIGHT - 0.5) * 2;
    for (let x = 0; x < FILM_WIDTH; x++, k++) {
      const vx = (x / FILM_WIDTH - 0.5) * 2;
      let value = Math.pow((clean[k] ?? 0) / 255, 0.9) * 1.08;
      value *= 1 - 0.36 * Math.pow(vx * vx * 0.6 + vy * vy * 0.4, 1.25);
      value = clamp(value + (random() - 0.5) * 0.05);
      film[k] = Math.round(value * 255);
    }
  }
  return { clean, film };
}

const painted = new Map<FilmKind, PaintedFilm>();

/** One sample film, upright, painted the first time it is asked for. */
export function paintedFilm(kind: FilmKind): PaintedFilm {
  let film = painted.get(kind);
  if (film === undefined) {
    film = paint(kind);
    painted.set(kind, film);
  }
  return film;
}
