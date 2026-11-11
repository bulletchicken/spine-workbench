// A small hand-made pass for unit tests and this area's gallery pages:
// every frame a Front and a Side film send, in the order a film pass and
// the chain pass emit them. The spine is a gentle curve of 18 bodies, C7
// to L5, on a film of 320 by 960 pixels. On Front, the search's guess for
// T12 runs 9 pixels too low and the locked chain puts it right, so one
// outline moves at lock-in. Side has no pedicle rings and one warning.
//
// Every call builds fresh frames, so a test may keep or transfer them.

import { TEMPLATE_LEVELS } from '@/shapes';
import type { ChainBody, ConfidenceBand, Film, Level, MapKind, PreviewTile, Px, Stage, TileEncoding, WatchFrame } from '@/shapes';

export const TEST_FILM_SIZE = { width: 320, height: 960 } as const;

/** The strip's rows and columns, and where it starts and how long it runs on the film. */
export const TEST_STRIP = { rows: 300, columns: 33, rowPx: 2.4, columnPx: 2.5, top: 100, length: 720 } as const;

/** The level whose guess on Front is wrong until lock-in. */
export const TEST_MOVED_LEVEL: Level = 'T12';

const LEVELS = TEMPLATE_LEVELS.standard;
const BODY = { firstTop: 112, pitch: 39, height: 28, halfWidth: 16 } as const;
const MOVED_BY = 9;
const BLOCK_ROWS = 64;

type TestFilm = 'front' | 'side';

const SPINES: Record<TestFilm, { centreX: number; bend: number }> = {
  front: { centreX: 160, bend: 18 },
  side: { centreX: 170, bend: -14 },
};

/** Every frame one film sends in a pass, in the order they are emitted. */
export function testFrames(film: TestFilm): WatchFrame[] {
  return [...evidenceStage(film), ...centreLineStage(film), ...stripStage(film), ...chainStages(film)];
}

/** The record of a test film as the case would hold it: upright, nothing confirmed yet. */
export function testFilm(film: TestFilm): Film {
  return {
    kind: film,
    image: { path: `films/${film}.png`, width: TEST_FILM_SIZE.width, height: TEST_FILM_SIZE.height, bitsStored: 12 },
    source: 'png',
    keptTags: null,
    values: { offset: 0, inverted: false, invertedByUser: false, modality: { kind: 'linear', slope: 1, intercept: 0 } },
    flags: [],
    blanked: [],
    geometry: {
      mmPerPx: null,
      mmPerPxVertical: null,
      scale: null,
      quarterTurns: 0,
      imageRight: null,
      confirmed: { upright: false, leftRight: false, facing: false, scale: false, polarity: false, privacy: false },
    },
  };
}

/** The film's locked bodies, top to bottom. */
export function testBodies(film: TestFilm): ChainBody[] {
  return LEVELS.map((level, index) => {
    const top = BODY.firstTop + index * BODY.pitch;
    return { level, sup: across(film, top, BODY.halfWidth), inf: across(film, top + BODY.height, BODY.halfWidth), inferred: false };
  });
}

function axisX(film: TestFilm, y: number): number {
  const { centreX, bend } = SPINES[film];
  return centreX + bend * Math.sin((Math.PI * (y - TEST_STRIP.top)) / TEST_STRIP.length);
}

function axisSlope(film: TestFilm, y: number): number {
  const { bend } = SPINES[film];
  return ((bend * Math.PI) / TEST_STRIP.length) * Math.cos((Math.PI * (y - TEST_STRIP.top)) / TEST_STRIP.length);
}

function across(film: TestFilm, y: number, halfWidth: number): { a: Px; b: Px } {
  const x = axisX(film, y);
  return { a: [x - halfWidth, y], b: [x + halfWidth, y] };
}

function bodyAt(y: number): number | null {
  const index = Math.floor((y - BODY.firstTop) / BODY.pitch);
  if (index < 0 || index >= LEVELS.length) return null;
  const top = BODY.firstTop + index * BODY.pitch;
  return y >= top && y <= top + BODY.height ? index : null;
}

function rowY(row: number): number {
  return TEST_STRIP.top + row * TEST_STRIP.rowPx;
}

function frameTile(width: number, height: number, step: number, encoding: TileEncoding, value: (x: number, y: number) => number | [number, number]): PreviewTile {
  const pair = encoding === 'signed-pair';
  const data = new Uint8Array(width * height * (pair ? 2 : 1));
  for (let j = 0; j < height; j++) {
    for (let i = 0; i < width; i++) {
      const cell = j * width + i;
      const v = value(step / 2 + i * step, step / 2 + j * step);
      if (Array.isArray(v)) {
        data[cell * 2] = v[0];
        data[cell * 2 + 1] = v[1];
      } else {
        data[cell] = v;
      }
    }
  }
  return { width, height, origin: [step / 2, step / 2], stepAcross: [step, 0], stepDown: [0, step], encoding, data };
}

function unit(v: number): number {
  return Math.round(255 * Math.min(1, Math.max(0, v)));
}

function edgeNear(film: TestFilm, x: number, y: number): number {
  const index = Math.round((y - BODY.firstTop - BODY.height / 2) / BODY.pitch);
  if (index < 0 || index >= LEVELS.length) return 0;
  const top = BODY.firstTop + index * BODY.pitch;
  const dx = Math.abs(x - axisX(film, y));
  if (dx > BODY.halfWidth + 6 || y < top - 6 || y > top + BODY.height + 6) return 0;
  const distance = Math.min(Math.abs(y - top), Math.abs(y - top - BODY.height), Math.abs(dx - BODY.halfWidth));
  return Math.exp(-((distance / 3) ** 2));
}

function ridgeNear(film: TestFilm, x: number, y: number): number {
  if (y < TEST_STRIP.top || y > TEST_STRIP.top + TEST_STRIP.length) return 0;
  return Math.exp(-(((x - axisX(film, y)) / 20) ** 2));
}

function ringNear(film: TestFilm, x: number, y: number): number {
  let best = Infinity;
  for (let index = 0; index < LEVELS.length; index++) {
    const middle = BODY.firstTop + index * BODY.pitch + BODY.height / 2;
    for (const side of [-1, 1]) best = Math.min(best, Math.hypot(x - axisX(film, middle) - side * 10, y - middle));
  }
  return Math.exp(-(((best - 4) / 1.5) ** 2));
}

function evidenceStage(film: TestFilm): WatchFrame[] {
  const maps: { map: MapKind; final: boolean; scaleMm: number | null; tile: PreviewTile }[] = [
    { map: 'image', final: true, scaleMm: null, tile: frameTile(32, 96, 10, 'unit', (_x, y) => unit(y / TEST_FILM_SIZE.height)) },
    { map: 'validity', final: true, scaleMm: null, tile: frameTile(32, 96, 10, 'mask', () => 255) },
    { map: 'phase-edge', final: false, scaleMm: 2, tile: frameTile(32, 96, 10, 'unit', (x, y) => unit(edgeNear(film, x, y) / 2)) },
    { map: 'phase-edge', final: true, scaleMm: null, tile: frameTile(32, 96, 10, 'unit', (x, y) => unit(edgeNear(film, x, y))) },
    { map: 'gradient-edge', final: true, scaleMm: null, tile: frameTile(32, 96, 10, 'unit', (x, y) => unit(edgeNear(film, x, y) * 0.8)) },
    { map: 'ridge', final: false, scaleMm: 4, tile: frameTile(32, 96, 10, 'signed-pair', (x, y) => [unit(ridgeNear(film, x, y)), 0]) },
    { map: 'ridge', final: true, scaleMm: null, tile: frameTile(32, 96, 10, 'unit', (x, y) => unit(ridgeNear(film, x, y))) },
    { map: 'valley', final: true, scaleMm: null, tile: frameTile(32, 96, 10, 'unit', () => 0) },
  ];
  if (film === 'front') {
    maps.push(
      { map: 'ring', final: false, scaleMm: 6, tile: frameTile(32, 96, 10, 'unit', (x, y) => unit(ringNear(film, x, y) / 2)) },
      { map: 'ring', final: true, scaleMm: null, tile: frameTile(32, 96, 10, 'unit', (x, y) => unit(ringNear(film, x, y))) },
    );
  }
  return [
    { kind: 'stage-started', film, stage: 'evidence' },
    ...maps.map((m, index): WatchFrame => ({ kind: 'evidence', film, stage: 'evidence', step: index + 1, steps: maps.length, ...m })),
  ];
}

function centreLineStage(film: TestFilm): WatchFrame[] {
  const stage: Stage = 'centre-line';
  const top = TEST_STRIP.top;
  const bottom = TEST_STRIP.top + TEST_STRIP.length;
  const wavefronts = [1, 2, 3, 4].map((n): WatchFrame => {
    const reach = top + (TEST_STRIP.length * n) / 4;
    return {
      kind: 'evidence',
      film,
      stage,
      map: 'wavefront',
      step: n + 1,
      steps: 5,
      final: n === 4,
      scaleMm: null,
      tile: frameTile(16, 48, 20, 'unit', (x, y) => (y >= top && y <= reach && Math.abs(x - axisX(film, y)) < 60 ? Math.round((254 * (y - top)) / TEST_STRIP.length) : 255)),
    };
  });
  const line = (every: number, wobble: number) =>
    Array.from({ length: TEST_STRIP.length / every + 1 }, (_, k): Px => [axisX(film, top + k * every) + (k % 2 === 0 ? -wobble : wobble), top + k * every]);
  return [
    { kind: 'stage-started', film, stage },
    {
      kind: 'end-zones',
      film,
      stage,
      top: { left: axisX(film, top) - 30, top: top - 20, right: axisX(film, top) + 30, bottom: top + 20 },
      bottom: { left: axisX(film, bottom) - 30, top: bottom - 20, right: axisX(film, bottom) + 30, bottom: bottom + 20 },
      topCropped: false,
      bottomCropped: false,
    },
    { kind: 'evidence', film, stage, map: 'ridge-heat', step: 1, steps: 5, final: true, scaleMm: null, tile: frameTile(16, 48, 20, 'unit', (x, y) => unit(ridgeNear(film, x, y))) },
    ...wavefronts,
    { kind: 'centre-line', film, stage, points: line(40, 3), refined: false },
    { kind: 'centre-line', film, stage, points: line(20, 0), refined: true },
    {
      kind: 'width',
      film,
      stage,
      samples: Array.from({ length: TEST_STRIP.length / 40 + 1 }, (_, k) => {
        const y = top + k * 40;
        return { left: [axisX(film, y) - 20, y] as Px, right: [axisX(film, y) + 20, y] as Px, confidence: 0.9 };
      }),
    },
  ];
}

function stripStage(film: TestFilm): WatchFrame[] {
  const stage: Stage = 'strip';
  const { rows, columns } = TEST_STRIP;
  const centres = Array.from({ length: rows }, (_, row): Px => [axisX(film, rowY(row)), rowY(row)]);
  const normals = Array.from({ length: rows }, (_, row): Px => {
    const slope = axisSlope(film, rowY(row));
    const length = Math.hypot(slope, 1);
    return [1 / length, -slope / length];
  });
  const blocks: WatchFrame[] = [];
  for (let firstRow = 0; firstRow < rows; firstRow += BLOCK_ROWS) {
    const blockRows = Math.min(BLOCK_ROWS, rows - firstRow);
    const image = new Uint8Array(blockRows * columns);
    const valid = new Uint8Array(Math.ceil((blockRows * columns) / 8));
    for (let r = 0; r < blockRows; r++) {
      const bone = bodyAt(rowY(firstRow + r)) !== null;
      for (let c = 0; c < columns; c++) {
        const cell = r * columns + c;
        image[cell] = bone ? 200 - 3 * Math.abs(c - (columns - 1) / 2) : 50;
        const usable = !(firstRow + r < 3 && (c === 0 || c === columns - 1));
        if (usable) valid[cell >> 3] = (valid[cell >> 3] ?? 0) | (1 << (cell & 7));
      }
    }
    blocks.push({ kind: 'strip-rows', film, stage, firstRow, rows: blockRows, columns, image, valid });
  }
  const profile = Array.from({ length: rows }, (_, row) => (bodyAt(rowY(row)) === null ? -0.5 : 0.8));
  const mean = profile.reduce((sum, v) => sum + v, 0) / rows;
  const frames: WatchFrame[] = [
    { kind: 'stage-started', film, stage },
    { kind: 'strip-shape', film, stage, centres, normals, columnPx: TEST_STRIP.columnPx, columns },
    ...blocks,
    { kind: 'barcode', film, stage, values: littleEndian(profile), highPass: littleEndian(profile.map((v) => v - mean)) },
  ];
  if (film === 'side') frames.push({ kind: 'warning', film, stage, warning: { code: 'scale-assumed', film, level: null, region: null } });
  return frames;
}

function littleEndian(values: number[]): Uint8Array {
  const bytes = new Uint8Array(values.length * 4);
  const view = new DataView(bytes.buffer);
  values.forEach((v, i) => {
    view.setFloat32(i * 4, v, true);
  });
  return bytes;
}

function chainStages(film: TestFilm): WatchFrame[] {
  const bodies = testBodies(film);
  const movedIndex = LEVELS.indexOf(TEST_MOVED_LEVEL);
  const guess = (body: ChainBody, index: number): ChainBody =>
    film === 'front' && index === movedIndex ? { ...body, inf: { a: [body.inf.a[0], body.inf.a[1] + MOVED_BY], b: [body.inf.b[0], body.inf.b[1] + MOVED_BY] } } : body;
  const sweeps = bodies.map((body, reached): WatchFrame => {
    const bin = Math.round((255 * (body.inf.a[1] - TEST_STRIP.top)) / TEST_STRIP.length);
    return {
      kind: 'sweep',
      film,
      stage: 'chain',
      level: body.level,
      heat: Uint8Array.from({ length: 256 }, (_, b) => unit(Math.exp(-(((b - bin) / 6) ** 2)))),
      tentative: bodies.slice(0, reached + 1).map(guess),
    };
  });
  const band = (index: number): ConfidenceBand => (index === movedIndex ? 'low' : index === 5 || index === 15 ? 'medium' : 'high');
  const score = { low: 0.4, medium: 0.7, high: 0.93 } as const;
  const levels = bodies.map((body) => body.level);
  return [
    { kind: 'stage-started', film, stage: 'chain' },
    ...sweeps,
    ...(['standard', 'extra-thoracic', 'extra-lumbar', 'missing-thoracic', 'missing-lumbar'] as const).map(
      (template, i): WatchFrame => ({ kind: 'template-scored', film, stage: 'chain', template, score: 10 + i, chance: i === 0 ? 0.8 : 0.05 }),
    ),
    { kind: 'chain-locked', film, stage: 'chain', bodies: bodies.slice().reverse() },
    { kind: 'stage-started', film, stage: 'naming' },
    {
      kind: 'levels-named',
      film,
      stage: 'naming',
      naming: {
        template: 'standard',
        chosenBy: 'engine',
        alternatives: [
          { template: 'standard', chance: 0.8 },
          { template: 'missing-lumbar', chance: 0.05 },
        ],
        fromTop: levels,
        fromBottom: levels,
        flags: [],
        l5ConfirmedOn: null,
      },
      levels,
    },
    { kind: 'stage-started', film, stage: 'confidence' },
    { kind: 'confidence', film, stage: 'confidence', levels: bodies.map((body, i) => ({ level: body.level, score: score[band(i)], band: band(i) })) },
    {
      kind: 'outlines',
      film,
      stage: 'confidence',
      quads: bodies.map((body) => ({
        level: body.level,
        corners: [
          [body.sup.a[0] + 1, body.sup.a[1]],
          [body.sup.b[0] - 1, body.sup.b[1]],
          [body.inf.b[0] - 1, body.inf.b[1]],
          [body.inf.a[0] + 1, body.inf.a[1]],
        ],
      })),
      final: true,
    },
  ];
}
