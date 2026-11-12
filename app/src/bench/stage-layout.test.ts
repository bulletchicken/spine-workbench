import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import type { FilmKind } from '@/shapes';
import { CAPTION, CAPTION_GAP, CLEAR, FILMS_HEIGHT, FILM_ORDER, GAP, HEAD_GAP, TALLEST, TILE_SHARE, TUCK, captionSpot, stageLayout, type Spot } from './stage-layout';

type Rect = readonly [x: number, y: number, width: number, height: number];
const rect = (value: Rect | null, radius: Spot['radius']): Spot | null => value === null ? null : { x: value[0], y: value[1], width: value[2], height: value[3], radius };

const filmsRows = [
  ['1920 × 1080', 1280, 848, 48, 152, 248, 210, 420, [184, 418, 652, 886]],
  ['1440 × 900', 1280, 672, 48, 66, 162, 208, 416, [188, 420, 652, 884]],
  ['1100 × 760', 1004, 532, 48, 60, 156, 144, 288, [178, 346, 514, 682]],
  ['900 × 800', 868, 572, 96, 60, 204, 140, 280, [118, 282, 446, 610]],
  ['700 × 900', 668, 672, 96, 101, 245, 149, 298, [0, 173, 346, 519]],
  ['640 × 800', 608, 572, 96, 66, 210, 134, 268, [0, 158, 316, 474]],
] as const;

interface FindRow { windowSize: string; width: number; height: number; boxes: readonly [Rect, Rect | null, Rect | null, Rect | null] }
const findRows: readonly FindRow[] = [
  { windowSize: '1920 × 1080', width: 1280, height: 848, boxes: [[435, 0, 410, 820], null, null, null] },
  { windowSize: '1920 × 1080', width: 1280, height: 848, boxes: [[218, 0, 410, 820], [652, 0, 410, 820], null, null] },
  { windowSize: '1920 × 1080', width: 1280, height: 848, boxes: [[108, 0, 410, 820], [542, 0, 410, 820], [976, 0, 195, 390], [976, 430, 195, 390]] },
  { windowSize: '1440 × 900', width: 1280, height: 672, boxes: [[479, 0, 322, 644], null, null, null] },
  { windowSize: '1440 × 900', width: 1280, height: 672, boxes: [[306, 0, 322, 644], [652, 0, 322, 644], null, null] },
  { windowSize: '1440 × 900', width: 1280, height: 672, boxes: [[391, 0, 322, 644], null, [737, 0, 151, 302], null] },
  { windowSize: '1440 × 900', width: 1280, height: 672, boxes: [[391, 0, 322, 644], null, [737, 0, 151, 302], [737, 342, 151, 302]] },
  { windowSize: '1440 × 900', width: 1280, height: 672, boxes: [[218, 0, 322, 644], [564, 0, 322, 644], null, [910, 0, 151, 302]] },
  { windowSize: '1440 × 900', width: 1280, height: 672, boxes: [[218, 0, 322, 644], [564, 0, 322, 644], [910, 0, 151, 302], [910, 342, 151, 302]] },
  { windowSize: '1100 × 760', width: 1004, height: 532, boxes: [[376, 0, 252, 504], null, null, null] },
  { windowSize: '1100 × 760', width: 1004, height: 532, boxes: [[238, 0, 252, 504], [514, 0, 252, 504], null, null] },
  { windowSize: '1100 × 760', width: 1004, height: 532, boxes: [[306, 0, 252, 504], null, [582, 0, 116, 232], null] },
  { windowSize: '1100 × 760', width: 1004, height: 532, boxes: [[306, 0, 252, 504], null, [582, 0, 116, 232], [582, 272, 116, 232]] },
  { windowSize: '1100 × 760', width: 1004, height: 532, boxes: [[168, 0, 252, 504], [444, 0, 252, 504], null, [720, 0, 116, 232]] },
  { windowSize: '1100 × 760', width: 1004, height: 532, boxes: [[168, 0, 252, 504], [444, 0, 252, 504], [720, 0, 116, 232], [720, 272, 116, 232]] },
  { windowSize: '900 × 800', width: 868, height: 572, boxes: [[298, 0, 272, 544], null, null, null] },
  { windowSize: '900 × 800', width: 868, height: 572, boxes: [[150, 0, 272, 544], [446, 0, 272, 544], null, null] },
  { windowSize: '900 × 800', width: 868, height: 572, boxes: [[75, 0, 272, 544], [371, 0, 272, 544], [667, 0, 126, 252], [667, 292, 126, 252]] },
  { windowSize: '700 × 900', width: 668, height: 672, boxes: [[173, 0, 322, 644], null, null, null] },
  { windowSize: '700 × 900', width: 668, height: 672, boxes: [[0, 0, 322, 644], [346, 0, 322, 644], null, null] },
  { windowSize: '700 × 900', width: 668, height: 672, boxes: [[0, 70, 252, 504], [276, 70, 252, 504], [552, 70, 116, 232], [552, 342, 116, 232]] },
  { windowSize: '640 × 800', width: 608, height: 572, boxes: [[168, 0, 272, 544], null, null, null] },
  { windowSize: '640 × 800', width: 608, height: 572, boxes: [[20, 0, 272, 544], [316, 0, 272, 544], null, null] },
  { windowSize: '640 × 800', width: 608, height: 572, boxes: [[93, 0, 272, 544], null, [389, 0, 126, 252], null] },
  { windowSize: '640 × 800', width: 608, height: 572, boxes: [[93, 0, 272, 544], null, [389, 0, 126, 252], [389, 292, 126, 252]] },
  { windowSize: '640 × 800', width: 608, height: 572, boxes: [[0, 44, 228, 456], [252, 44, 228, 456], null, [504, 44, 104, 208]] },
  { windowSize: '640 × 800', width: 608, height: 572, boxes: [[0, 44, 228, 456], [252, 44, 228, 456], [504, 44, 104, 208], [504, 292, 104, 208]] },
];

describe('the stage layout', () => {
  for (const [windowSize, width, height, displayHeight, displayTop, y, w, h, xs] of filmsRows) {
    it(`puts all four Films spots at their expected pixels at ${windowSize}`, () => {
      const expected = Object.fromEntries(FILM_ORDER.map((kind, index) => [kind, rect([xs[index] ?? 0, y, w, h], 'large')]));
      for (const present of [[], ['front'], FILM_ORDER] as readonly (readonly FilmKind[])[]) {
        expect(stageLayout({ width, height, displayHeight, moment: 'films', present })).toEqual({ displayTop, spots: expected });
      }
    });
  }
  for (const { windowSize, width, height, boxes } of findRows) {
    const present = FILM_ORDER.filter((_kind, index) => boxes[index] !== null);
    it(`puts Find's ${present.join(', ')} at their expected pixels at ${windowSize}`, () => {
      const expected = Object.fromEntries(FILM_ORDER.map((kind, index) => [kind, rect(boxes[index] ?? null, index < 2 ? 'large' : 'medium')]));
      const layout = stageLayout({ width, height, displayHeight: 48, moment: 'find', present });
      expect(layout).toEqual({ displayTop: 0, spots: expected });
      for (const spot of Object.values(layout.spots)) if (spot !== null) {
        expect([spot.x, spot.y, spot.width, spot.height].every(Number.isInteger)).toBe(true);
        expect(spot.x).toBeGreaterThanOrEqual(0);
        expect(spot.x + spot.width).toBeLessThanOrEqual(width);
        expect(spot.y + spot.height + CAPTION).toBeLessThanOrEqual(height);
      }
    });
  }
  it('puts a lone right bending film at the top of its column', () => {
    const layout = stageLayout({ width: 608, height: 572, displayHeight: 96, moment: 'find', present: ['bend-right', 'front'] });
    expect(layout.spots['bend-right']).toEqual(rect([389, 0, 126, 252], 'medium'));
    expect(layout.spots['bend-left']).toBeNull();
  });
  it('keeps shrinking below the designed window and handles an empty stage', () => {
    for (const moment of ['films', 'find'] as const) for (const width of [0, 320, 540]) {
      const layout = stageLayout({ width, height: 0, displayHeight: 96, moment, present: [] });
      for (const spot of Object.values(layout.spots)) if (spot !== null) {
        expect(spot.width).toBeGreaterThanOrEqual(0);
        expect(spot.height).toBeGreaterThanOrEqual(0);
      }
    }
  });
  it('places a caption below its tile, centred', () => {
    expect(captionSpot({ x: 218, y: 0, width: 322, height: 644, radius: 'large' })).toEqual({ centreX: 379, top: 654 });
  });
  it('uses the design system’s own layout values', () => {
    const css = readFileSync(new URL('../look-and-feel/tokens.css', import.meta.url), 'utf8');
    for (const [name, value] of Object.entries({ '--tile-gap': GAP, '--caption-room': CAPTION, '--caption-gap': CAPTION_GAP, '--tuck-gap': TUCK, '--tile-max-h': TALLEST, '--tile-h-films': FILMS_HEIGHT, '--tile-share': TILE_SHARE, '--head-gap': HEAD_GAP, '--clear': CLEAR })) {
      expect(Number(new RegExp(`${name}:\\s*([\\d.]+)`).exec(css)?.[1]), name).toBe(value);
    }
  });
});
