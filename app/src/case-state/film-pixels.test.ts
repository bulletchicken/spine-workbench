import { describe, expect, it } from 'vitest';
import { createFilmPixels } from './film-pixels';

const film = (fill: number) => ({ pixels: new Uint16Array([fill, fill]), thumbnail: new Uint8Array([fill]) });

describe('film pixels', () => {
  it('holds no film and no opening at first', () => {
    const state = createFilmPixels().getState();
    expect(state.films).toEqual({});
    expect(state.openings).toEqual({ front: 0, side: 0, 'bend-left': 0, 'bend-right': 0 });
  });

  it('keeps each opened film by slot and counts every opening of that slot', () => {
    const store = createFilmPixels();
    const first = film(1);
    const second = film(2);
    store.getState().opened('side', first);
    expect(store.getState().films.side).toBe(first);
    store.getState().opened('side', second);
    expect(store.getState().films.side).toBe(second);
    expect(store.getState().openings).toEqual({ front: 0, side: 2, 'bend-left': 0, 'bend-right': 0 });
  });

  it('lets a retired film go and keeps its count, so the next opening reads 2', () => {
    const store = createFilmPixels();
    store.getState().opened('front', film(1));
    store.getState().retired('front');
    expect(store.getState().films.front).toBeUndefined();
    expect(store.getState().openings.front).toBe(1);
    store.getState().opened('front', film(3));
    expect(store.getState().openings.front).toBe(2);
  });

  it('tells no one when a slot with no film is retired', () => {
    const store = createFilmPixels();
    let told = 0;
    store.subscribe(() => {
      told += 1;
    });
    store.getState().retired('bend-left');
    expect(told).toBe(0);
  });
});
