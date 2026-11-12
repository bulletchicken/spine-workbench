// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { flightFrom, fly } from './fly-tiles';
import type { Spot } from './stage-layout';

const films: Spot = { x: 188, y: 162, width: 208, height: 416, radius: 'large' };
const find: Spot = { x: 218, y: 0, width: 322, height: 644, radius: 'large' };

function ended(tile: HTMLElement, propertyName = 'transform', type = 'transitionend') {
  tile.dispatchEvent(Object.assign(new Event(type, { bubbles: true }), { propertyName }));
}

describe('tiles in flight', () => {
  it('makes the new box’s centred scale meet the old edges, to Find and back', () => {
    expect(flightFrom(films, find)).toEqual({ x: 131, y: 48, s: 208 / 322 });
    expect(flightFrom(find, films)).toEqual({ x: 275, y: 114, s: 322 / 208 });
    for (const [was, now] of [[films, find], [find, films]] as const) {
      const start = flightFrom(was, now);
      expect(start.x + now.width * (1 - start.s) / 2).toBeCloseTo(was.x);
      expect(start.y + now.height * (1 - start.s) / 2).toBeCloseTo(was.y);
      expect(now.width * start.s).toBeCloseTo(was.width);
      expect(now.height * start.s).toBeCloseTo(was.height);
    }
  });
  it('leaves every tile at scale 1 and keeps the pointer away until the last transform ends', () => {
    const tiles = document.createElement('div');
    const front = document.createElement('div');
    const side = document.createElement('div');
    tiles.append(front, side);
    fly(tiles, new Map([[front, flightFrom(films, find)], [side, flightFrom(films, find)]]), new Map([[front, find], [side, find]]));
    expect(tiles.classList.contains('still')).toBe(false);
    expect(tiles.hasAttribute('data-flying')).toBe(true);
    expect(front.style.getPropertyValue('--s')).toBe('1');
    expect(side.style.getPropertyValue('--s')).toBe('1');
    expect(front.style.getPropertyValue('--x')).toBe('218');
    ended(front, 'border-radius');
    expect(tiles.hasAttribute('data-flying')).toBe(true);
    ended(front);
    expect(tiles.hasAttribute('data-flying')).toBe(true);
    ended(side);
    expect(tiles.hasAttribute('data-flying')).toBe(false);
  });
  it('cancels a previous flight on resize and does not wait on an unchanged tile', () => {
    const tiles = document.createElement('div');
    const tile = document.createElement('div');
    tiles.append(tile);
    fly(tiles, new Map([[tile, flightFrom(films, find)]]), new Map([[tile, find]]));
    fly(tiles, new Map(), new Map());
    expect(tiles.hasAttribute('data-flying')).toBe(false);
    fly(tiles, new Map([[tile, { x: find.x, y: find.y, s: 1 }]]), new Map([[tile, find]]));
    expect(tiles.hasAttribute('data-flying')).toBe(false);
  });
  it('releases a cancelled transform too', () => {
    const tiles = document.createElement('div');
    const tile = document.createElement('div');
    tiles.append(tile);
    fly(tiles, new Map([[tile, flightFrom(films, find)]]), new Map([[tile, find]]));
    ended(tile, 'transform', 'transitioncancel');
    expect(tiles.hasAttribute('data-flying')).toBe(false);
  });
});
