import type { FilmKind } from '@/shapes';
import type { Moment } from './Stage';

export const FILM_ORDER: readonly FilmKind[] = ['front', 'side', 'bend-left', 'bend-right'];

export interface Spot {
  x: number;
  y: number;
  width: number;
  height: number;
  radius: 'large' | 'medium' | 'small';
}

export interface StageInput {
  width: number;
  height: number;
  displayHeight: number;
  moment: Moment;
  present: readonly FilmKind[];
  railHeight?: number;
  railGap?: number;
}

export interface StageLayout {
  displayTop: number;
  spots: Record<FilmKind, Spot | null>;
}

export const RAIL_HEIGHT = 132;
export const RAIL_GAP = 12;
export const GAP = 24;
export const CAPTION = 28;
export const CAPTION_GAP = 10;
export const TUCK = 12;
export const TALLEST = 820;
export const FILMS_HEIGHT = 420;
export const TILE_SHARE = 0.62;
export const HEAD_GAP = 48;
export const CLEAR = 60;

const down2 = (value: number) => Math.max(0, Math.floor(value / 2) * 2);
const down4 = (value: number) => Math.max(0, Math.floor(value / 4) * 4);

/** The same row in Films; only the films present, centred together, in Find. */
export function stageLayout({ width, height, displayHeight, moment, present, railHeight = RAIL_HEIGHT, railGap = RAIL_GAP }: StageInput): StageLayout {
  const spots: StageLayout['spots'] = { front: null, side: null, 'bend-left': null, 'bend-right': null };
  if (moment === 'three') {
    const ordered = FILM_ORDER.filter((kind) => present.includes(kind));
    const n = ordered.length;
    if (n === 0) return { displayTop: 0, spots };
    const h = Math.min(railHeight, Math.max(0, (height - (n - 1) * railGap) / n));
    const y = (height - n * h - (n - 1) * railGap) / 2;
    ordered.forEach((kind, index) => { spots[kind] = { x: 0, y: y + index * (h + railGap), width: h / 2, height: h, radius: 'small' }; });
    return { displayTop: 0, spots };
  }
  if (moment === 'films') {
    const head = displayHeight + HEAD_GAP;
    const tileHeight = down2(Math.min(height * TILE_SHARE, FILMS_HEIGHT, height - head - CAPTION - 2 * CLEAR, (width - GAP * 3) / 2));
    const tileWidth = tileHeight / 2;
    const displayTop = Math.max(0, Math.round((height - head - tileHeight - CAPTION) / 2));
    const x = Math.floor((width - (tileWidth * 4 + GAP * 3)) / 2);
    FILM_ORDER.forEach((kind, index) => {
      spots[kind] = { x: x + index * (tileWidth + GAP), y: displayTop + head, width: tileWidth, height: tileHeight, radius: 'large' };
    });
    return { displayTop, spots };
  }

  const big = FILM_ORDER.filter((kind) => (kind === 'front' || kind === 'side') && present.includes(kind));
  const bends = FILM_ORDER.filter((kind) => (kind === 'bend-left' || kind === 'bend-right') && present.includes(kind));
  const count = big.length;
  const column = bends.length > 0 ? 1 : 0;
  if (count + column === 0) return { displayTop: 0, spots };
  const bigHeight = down4(Math.min(height - CAPTION, TALLEST, (width - GAP * (count - 1 + column) + ((CAPTION + TUCK) / 4) * column) / (count / 2 + column / 4)));
  const bigWidth = bigHeight / 2;
  const smallHeight = Math.max(0, (bigHeight - CAPTION - TUCK) / 2);
  const smallWidth = smallHeight / 2;
  const rowWidth = count * bigWidth + column * smallWidth + GAP * (count - 1 + column);
  const x = Math.floor((width - rowWidth) / 2);
  const top = Math.max(0, Math.round((height - CAPTION - bigHeight) / 2));
  big.forEach((kind, index) => {
    spots[kind] = { x: x + index * (bigWidth + GAP), y: top, width: bigWidth, height: bigHeight, radius: 'large' };
  });
  bends.forEach((kind, index) => {
    spots[kind] = { x: x + count * (bigWidth + GAP), y: top + index * (smallHeight + CAPTION + TUCK), width: smallWidth, height: smallHeight, radius: 'medium' };
  });
  return { displayTop: 0, spots };
}

export function captionSpot(spot: Spot): { centreX: number; top: number } {
  return { centreX: spot.x + spot.width / 2, top: spot.y + spot.height + CAPTION_GAP };
}
