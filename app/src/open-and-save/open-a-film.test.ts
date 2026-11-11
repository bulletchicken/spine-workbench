import { describe, expect, it } from 'vitest';
import { createFilmPixels } from '@/case-state/film-pixels';
import type { FilmKind } from '@/shapes';
import { EngineFailure } from '@/engine-room/engine-client';
import { openAFilm, type FilmOpener, type OpenedFilm } from './open-a-film';

/** A promise the test settles by hand. */
function deferred<T>() {
  let resolve: (value: T) => void = () => undefined;
  let reject: (reason: unknown) => void = () => undefined;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const FACTS = {} as OpenedFilm['facts'];

/** A fake engine that records its calls; it can refuse the bytes or fail the pixels. */
function fakeEngine(options: { refuse?: boolean; pixelsFail?: boolean } = {}) {
  const calls: string[] = [];
  const engine: FilmOpener = {
    openFilm: (kind: FilmKind, bytes: ArrayBuffer) => {
      calls.push(`open ${kind} ${String(bytes.byteLength)}`);
      if (options.refuse === true) {
        return Promise.reject(new EngineFailure({ code: 'film-unknown-format', message: 'not a film', detail: null }));
      }
      return Promise.resolve({ facts: FACTS, thumbnail: new Uint8Array([1, 2, 3]) });
    },
    displayPixels: (kind: FilmKind) => {
      calls.push(`pixels ${kind}`);
      if (options.pixelsFail === true) return Promise.reject(new Error('pixels failed'));
      return Promise.resolve(new Uint16Array([7, 7, 7, 7]));
    },
  };
  return { engine, calls };
}

const bytes = (length: number): ArrayBuffer => new ArrayBuffer(length);

describe('open a film', () => {
  it('opens the film, reads its pixels and keeps them with its thumbnail for the slot', async () => {
    const films = createFilmPixels();
    const { engine, calls } = fakeEngine();
    const opened = await openAFilm(engine, 'front', bytes(5), null, films);
    expect(calls).toEqual(['open front 5', 'pixels front']);
    expect(opened.thumbnail).toEqual(new Uint8Array([1, 2, 3]));
    expect(films.getState().films.front).toEqual({ pixels: new Uint16Array([7, 7, 7, 7]), thumbnail: new Uint8Array([1, 2, 3]) });
    expect(films.getState().openings.front).toBe(1);
    expect(films.getState().openings.side).toBe(0);
  });

  it('counts each opening of a slot, so reopening a film raises its count again', async () => {
    const films = createFilmPixels();
    const { engine } = fakeEngine();
    await openAFilm(engine, 'side', bytes(1), null, films);
    await openAFilm(engine, 'side', bytes(1), null, films);
    expect(films.getState().openings.side).toBe(2);
  });

  it('leaves the store as it was when the engine refuses the bytes', async () => {
    const films = createFilmPixels();
    await openAFilm(fakeEngine().engine, 'bend-left', bytes(1), null, films);
    const kept = films.getState().films['bend-left'];
    const refusing = fakeEngine({ refuse: true });
    await expect(openAFilm(refusing.engine, 'bend-left', bytes(1), null, films)).rejects.toBeInstanceOf(EngineFailure);
    expect(films.getState().films['bend-left']).toBe(kept);
    expect(films.getState().openings['bend-left']).toBe(1);
    expect(refusing.calls).toEqual(['open bend-left 1']);
  });

  it('when the pixels fail after the film opened, lets the old pixels of that slot go and counts nothing', async () => {
    const films = createFilmPixels();
    await openAFilm(fakeEngine().engine, 'front', bytes(1), null, films);
    await expect(openAFilm(fakeEngine({ pixelsFail: true }).engine, 'front', bytes(1), null, films)).rejects.toThrow('pixels failed');
    expect(films.getState().films.front).toBeUndefined();
    expect(films.getState().openings.front).toBe(1);
  });

  it('when two openings of one slot overlap, only the newer one keeps its pixels', async () => {
    const films = createFilmPixels();
    const olderEngine = { open: deferred<OpenedFilm>(), pixels: deferred<Uint16Array>() };
    const newerEngine = { open: deferred<OpenedFilm>(), pixels: deferred<Uint16Array>() };
    let olderAskedForPixels = false;
    const older: FilmOpener = {
      openFilm: () => olderEngine.open.promise,
      displayPixels: () => {
        olderAskedForPixels = true;
        return olderEngine.pixels.promise;
      },
    };
    const newer: FilmOpener = {
      openFilm: () => newerEngine.open.promise,
      displayPixels: () => newerEngine.pixels.promise,
    };
    const first = openAFilm(older, 'side', bytes(1), null, films);
    const second = openAFilm(newer, 'side', bytes(2), null, films);
    newerEngine.open.resolve({ facts: FACTS, thumbnail: new Uint8Array([9]) });
    newerEngine.pixels.resolve(new Uint16Array([9]));
    await second;
    olderEngine.open.resolve({ facts: FACTS, thumbnail: new Uint8Array([8]) });
    olderEngine.pixels.resolve(new Uint16Array([8]));
    await expect(first).rejects.toMatchObject({ error: { code: 'pass-cancelled' } });
    // Overtaken before its pixels, it never asks for them: the engine would
    // answer with the newer film's.
    expect(olderAskedForPixels).toBe(false);
    expect(films.getState().films.side?.pixels).toEqual(new Uint16Array([9]));
    expect(films.getState().openings.side).toBe(1);
  });

  it('openings of two different slots that overlap leave each other alone: both keep their pixels', async () => {
    const films = createFilmPixels();
    const frontOpen = deferred<OpenedFilm>();
    const slowFront: FilmOpener = { openFilm: () => frontOpen.promise, displayPixels: () => Promise.resolve(new Uint16Array([1])) };
    const fastSide: FilmOpener = {
      openFilm: () => Promise.resolve({ facts: FACTS, thumbnail: new Uint8Array([2]) }),
      displayPixels: () => Promise.resolve(new Uint16Array([2])),
    };
    const front = openAFilm(slowFront, 'front', bytes(1), null, films);
    await openAFilm(fastSide, 'side', bytes(1), null, films);
    frontOpen.resolve({ facts: FACTS, thumbnail: new Uint8Array([1]) });
    await front;
    expect(films.getState().films.front).toEqual({ pixels: new Uint16Array([1]), thumbnail: new Uint8Array([1]) });
    expect(films.getState().films.side).toEqual({ pixels: new Uint16Array([2]), thumbnail: new Uint8Array([2]) });
    expect(films.getState().openings.front).toBe(1);
    expect(films.getState().openings.side).toBe(1);
  });

  it('when a newer opening of the slot is refused, an older one that finishes still rejects and keeps nothing', async () => {
    const films = createFilmPixels();
    const older = deferred<OpenedFilm>();
    const newer = deferred<OpenedFilm>();
    const olderOpener: FilmOpener = { openFilm: () => older.promise, displayPixels: () => Promise.resolve(new Uint16Array([8])) };
    const newerOpener: FilmOpener = { openFilm: () => newer.promise, displayPixels: () => Promise.resolve(new Uint16Array([9])) };
    const first = openAFilm(olderOpener, 'side', bytes(1), null, films);
    const second = openAFilm(newerOpener, 'side', bytes(2), null, films);
    newer.reject(new EngineFailure({ code: 'film-unknown-format', message: 'not a film', detail: null }));
    await expect(second).rejects.toBeInstanceOf(EngineFailure);
    older.resolve({ facts: FACTS, thumbnail: new Uint8Array([8]) });
    await expect(first).rejects.toMatchObject({ error: { code: 'pass-cancelled' } });
    expect(films.getState().films.side).toBeUndefined();
    expect(films.getState().openings.side).toBe(0);
  });

  it('an opening overtaken while its pixels are on the way rejects and keeps nothing, even when they arrive', async () => {
    const films = createFilmPixels();
    const olderPixels = deferred<Uint16Array>();
    const newerOpen = deferred<OpenedFilm>();
    const pixelsAsked = deferred<undefined>();
    const older: FilmOpener = {
      openFilm: () => Promise.resolve({ facts: FACTS, thumbnail: new Uint8Array([8]) }),
      displayPixels: () => {
        pixelsAsked.resolve(undefined);
        return olderPixels.promise;
      },
    };
    const newer: FilmOpener = { openFilm: () => newerOpen.promise, displayPixels: () => Promise.resolve(new Uint16Array([9])) };
    const first = openAFilm(older, 'side', bytes(1), null, films);
    await pixelsAsked.promise;
    const second = openAFilm(newer, 'side', bytes(2), null, films);
    olderPixels.resolve(new Uint16Array([8]));
    await expect(first).rejects.toMatchObject({ error: { code: 'pass-cancelled' } });
    expect(films.getState().films.side).toBeUndefined();
    expect(films.getState().openings.side).toBe(0);
    newerOpen.resolve({ facts: FACTS, thumbnail: new Uint8Array([9]) });
    await second;
    expect(films.getState().films.side?.pixels).toEqual(new Uint16Array([9]));
    expect(films.getState().openings.side).toBe(1);
  });

  it('a late pixel failure of an overtaken opening leaves the newer opening its pixels', async () => {
    const films = createFilmPixels();
    const olderPixels = deferred<Uint16Array>();
    const pixelsAsked = deferred<undefined>();
    const older: FilmOpener = {
      openFilm: () => Promise.resolve({ facts: FACTS, thumbnail: new Uint8Array([8]) }),
      displayPixels: () => {
        pixelsAsked.resolve(undefined);
        return olderPixels.promise;
      },
    };
    const first = openAFilm(older, 'front', bytes(1), null, films);
    await pixelsAsked.promise;
    await openAFilm(fakeEngine().engine, 'front', bytes(2), null, films);
    olderPixels.reject(new EngineFailure({ code: 'pass-cancelled', message: 'replaced', detail: null }));
    await expect(first).rejects.toMatchObject({ error: { code: 'pass-cancelled' } });
    expect(films.getState().films.front).toEqual({ pixels: new Uint16Array([7, 7, 7, 7]), thumbnail: new Uint8Array([1, 2, 3]) });
    expect(films.getState().openings.front).toBe(1);
  });
});
