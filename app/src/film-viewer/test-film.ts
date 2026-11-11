// A film the viewer makes for itself, for its gallery page and for
// measuring how it moves: a full-length front view, 3000 by 7000 pixels,
// drawn as a stylised radiograph so it reads like a film at fit, with
// single bright pixels at known places so that anything drawn over it can
// be checked against exact pixel centres at any zoom. It is not anatomy
// and never stands in for a patient's film.

import type { Px } from '@/shapes';
import type { FilmSize } from './film-space';
import { floatToHalf } from './gpu/half-float';

export const TEST_FILM_SIZE: FilmSize = { width: 3000, height: 7000 };

export interface TestVertebra {
  name: string;
  /** Top left, top right, bottom right, bottom left, in film pixels. */
  corners: [Px, Px, Px, Px];
  centre: Px;
}

export interface TestFilm {
  size: FilmSize;
  /** Half-float display pixels, 0 black to 1 white. */
  pixels: Uint16Array;
  vertebrae: TestVertebra[];
  /** Pixels lit alone on a dark ground: the four corner pixels and a few inside. */
  probes: Px[];
}

const NAMES = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];

/** Grey levels the picture is quantised to before grain; fine enough that no band shows after window and level. */
const LEVELS = 4096;

const made = new Map<string, TestFilm>();

/** The test film at `size`, made once and kept. */
export function testFilm(size: FilmSize = TEST_FILM_SIZE): TestFilm {
  const key = `${String(size.width)}x${String(size.height)}`;
  const kept = made.get(key);
  if (kept !== undefined) return kept;
  const film = drawTestFilm(size);
  made.set(key, film);
  return film;
}

/**
 * The same film as it would arrive stored upside down, as bending films
 * sometimes do: every pixel turned half a turn, which is the pixels in
 * reverse order. Shown with two quarter turns it reads upright again.
 */
export function storedUpsideDown(film: TestFilm): TestFilm {
  const kept = turned.get(film);
  if (kept !== undefined) return kept;
  const { width, height } = film.size;
  const turn = ([x, y]: Px): Px => [width - 1 - x, height - 1 - y];
  const upsideDown: TestFilm = {
    size: film.size,
    pixels: film.pixels.slice().reverse(),
    vertebrae: film.vertebrae.map(({ name, corners, centre }) => ({
      name,
      corners: [turn(corners[0]), turn(corners[1]), turn(corners[2]), turn(corners[3])],
      centre: turn(centre),
    })),
    probes: film.probes.map(turn),
  };
  turned.set(film, upsideDown);
  return upsideDown;
}

const turned = new WeakMap<TestFilm, TestFilm>();

function drawTestFilm(size: FilmSize): TestFilm {
  const { width, height } = size;
  const canvas = new OffscreenCanvas(width, height);
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (context === null) throw new Error('No 2D canvas to draw the test film with');
  const k = width / 3000;
  const vertebrae = spine(width, height, k);

  context.fillStyle = '#000';
  context.fillRect(0, 0, width, height);
  body(context, width, height, k);
  ribs(context, vertebrae, k);
  pelvis(context, width, height, k);
  bones(context, vertebrae, k);
  marker(context, width, k);
  const probes = probePixels(context, width, height, k);

  return { size, pixels: toHalfFloats(context, width, height), vertebrae, probes };
}

/** Seventeen vertebrae down a gentle S, each a little taller than the one above. */
function spine(width: number, height: number, k: number): TestVertebra[] {
  const top = height * 0.2;
  const bottom = height * 0.74;
  const heights = NAMES.map((_, i) => 1 + i * 0.045);
  const total = heights.reduce((sum, h) => sum + h, 0);
  const gap = 0.3;
  const unit = (bottom - top) / (total + gap * NAMES.length);
  let y = top;
  return NAMES.map((name, i) => {
    const tall = (heights[i] ?? 1) * unit;
    const mid = y + tall / 2;
    const t = (mid - top) / (bottom - top);
    const x = width / 2 + width * 0.06 * Math.sin(t * Math.PI * 1.6 - 0.4);
    const tilt = 0.12 * Math.cos(t * Math.PI * 1.6 - 0.4);
    const half = (150 + i * 9) * k;
    const lift = Math.tan(tilt) * half;
    const corners: [Px, Px, Px, Px] = [
      [x - half, y + lift],
      [x + half, y - lift],
      [x + half, y + tall - lift],
      [x - half, y + tall + lift],
    ];
    y += tall + gap * unit;
    return { name, corners, centre: [x, mid] };
  });
}

function body(context: OffscreenCanvasRenderingContext2D, width: number, height: number, k: number) {
  context.save();
  context.filter = `blur(${String(60 * k)}px)`;
  const torso = context.createLinearGradient(0, 0, width, 0);
  torso.addColorStop(0, 'rgb(10,10,10)');
  torso.addColorStop(0.5, 'rgb(46,46,46)');
  torso.addColorStop(1, 'rgb(10,10,10)');
  context.fillStyle = torso;
  context.beginPath();
  context.ellipse(width / 2, height * 0.47, width * 0.4, height * 0.4, 0, 0, Math.PI * 2);
  context.fill();
  context.fillStyle = 'rgb(70,70,70)';
  context.beginPath();
  context.ellipse(width / 2, height * 0.05, width * 0.17, height * 0.07, 0, 0, Math.PI * 2);
  context.fill();
  context.restore();
}

function ribs(context: OffscreenCanvasRenderingContext2D, vertebrae: TestVertebra[], k: number) {
  context.save();
  context.filter = `blur(${String(5 * k)}px)`;
  context.strokeStyle = 'rgb(112,112,112)';
  context.lineCap = 'round';
  vertebrae.slice(0, 12).forEach((vertebra, i) => {
    const [x, y] = vertebra.centre;
    const reach = (700 + 40 * Math.min(i, 7)) * k;
    context.lineWidth = (34 - i) * k;
    for (const side of [-1, 1]) {
      context.beginPath();
      context.moveTo(x + side * 130 * k, y);
      context.bezierCurveTo(x + side * reach * 0.9, y - 260 * k, x + side * reach * 1.2, y + 120 * k, x + side * reach, y + 520 * k);
      context.stroke();
    }
  });
  context.restore();
}

function pelvis(context: OffscreenCanvasRenderingContext2D, width: number, height: number, k: number) {
  context.save();
  context.filter = `blur(${String(14 * k)}px)`;
  context.fillStyle = 'rgb(96,96,96)';
  for (const side of [-1, 1]) {
    context.beginPath();
    context.ellipse(width / 2 + side * 560 * k, height * 0.8, 430 * k, 560 * k, side * -0.35, 0, Math.PI * 2);
    context.fill();
    context.fillStyle = 'rgb(130,130,130)';
    context.beginPath();
    context.arc(width / 2 + side * 640 * k, height * 0.9, 150 * k, 0, Math.PI * 2);
    context.fill();
    context.fillStyle = 'rgb(96,96,96)';
  }
  context.fillStyle = 'rgb(118,118,118)';
  context.beginPath();
  context.moveTo(width / 2 - 250 * k, height * 0.752);
  context.quadraticCurveTo(width / 2, height * 0.745, width / 2 + 250 * k, height * 0.752);
  context.quadraticCurveTo(width / 2 + 120 * k, height * 0.83, width / 2, height * 0.86);
  context.quadraticCurveTo(width / 2 - 120 * k, height * 0.83, width / 2 - 250 * k, height * 0.752);
  context.fill();
  context.restore();
}

function bones(context: OffscreenCanvasRenderingContext2D, vertebrae: TestVertebra[], k: number) {
  context.save();
  context.filter = `blur(${String(3 * k)}px)`;
  for (const { corners, centre } of vertebrae) {
    const body = context.createLinearGradient(corners[0][0], 0, corners[1][0], 0);
    body.addColorStop(0, 'rgb(150,150,150)');
    body.addColorStop(0.5, 'rgb(196,196,196)');
    body.addColorStop(1, 'rgb(150,150,150)');
    context.fillStyle = body;
    context.beginPath();
    corners.forEach(([x, y], i) => {
      if (i === 0) context.moveTo(x, y);
      else context.lineTo(x, y);
    });
    context.closePath();
    context.fill();
    context.strokeStyle = 'rgb(225,225,225)';
    context.lineWidth = 9 * k;
    context.stroke();
    // Pedicle rings.
    const across = (corners[1][0] - corners[0][0]) * 0.3;
    context.strokeStyle = 'rgb(232,232,232)';
    context.lineWidth = 12 * k;
    for (const side of [-1, 1]) {
      context.beginPath();
      context.ellipse(centre[0] + side * across, centre[1] - 20 * k, 32 * k, 46 * k, 0, 0, Math.PI * 2);
      context.stroke();
    }
  }
  context.restore();
}

/** The lead L in its ring, up and to the right, as a front film carries it. */
function marker(context: OffscreenCanvasRenderingContext2D, width: number, k: number) {
  context.save();
  context.strokeStyle = 'rgb(250,250,250)';
  context.fillStyle = 'rgb(250,250,250)';
  context.lineWidth = 18 * k;
  context.beginPath();
  context.arc(width * 0.82, 760 * k, 100 * k, 0, Math.PI * 2);
  context.stroke();
  context.font = `700 ${String(130 * k)}px sans-serif`;
  context.textAlign = 'center';
  context.textBaseline = 'middle';
  context.fillText('L', width * 0.82, 768 * k);
  context.restore();
}

/** Single white pixels on black squares: the film's four corner pixels and three inside. */
function probePixels(context: OffscreenCanvasRenderingContext2D, width: number, height: number, k: number): Px[] {
  const inside: Px[] = [
    [Math.round(width * 0.25), Math.round(400 * k)],
    [Math.round(width * 0.5), Math.round(height * 0.12)],
    [Math.round(width * 0.25) + 3, Math.round(400 * k) + 2],
  ];
  const probes: Px[] = [[0, 0], [width - 1, 0], [width - 1, height - 1], [0, height - 1], ...inside];
  context.save();
  for (const [x, y] of probes) {
    context.fillStyle = '#000';
    context.fillRect(x - 12, y - 12, 25, 25);
  }
  context.fillStyle = '#fff';
  for (const [x, y] of probes) context.fillRect(x, y, 1, 1);
  context.restore();
  return probes;
}

/** The picture's grey levels as half floats, with a little grain so window and level never show bands. */
function toHalfFloats(context: OffscreenCanvasRenderingContext2D, width: number, height: number): Uint16Array {
  const halves = new Uint16Array(LEVELS);
  for (let i = 0; i < LEVELS; i += 1) halves[i] = floatToHalf(i / (LEVELS - 1));
  const out = new Uint16Array(width * height);
  const band = 256;
  let seed = 0x2545f491;
  for (let top = 0; top < height; top += band) {
    const rows = Math.min(band, height - top);
    const { data } = context.getImageData(0, top, width, rows);
    for (let i = 0, p = top * width; i < width * rows; i += 1, p += 1) {
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
      const grain = (seed / 0x100000000 - 0.5) * 0.02;
      const value = (data[i * 4] ?? 0) / 255 + grain;
      const level = Math.min(LEVELS - 1, Math.max(0, Math.round(value * (LEVELS - 1))));
      out[p] = halves[level] ?? 0;
    }
  }
  return out;
}
