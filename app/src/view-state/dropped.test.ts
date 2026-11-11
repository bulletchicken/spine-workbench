import { describe, expect, it } from 'vitest';
import { createDropped } from './dropped';

const file = (...values: number[]): Blob => new Blob([new Uint8Array(values)]);

describe('dropped files', () => {
  it('keeps films in the order they were dropped, and each is taken once', () => {
    const dropped = createDropped();
    const first = { kind: null, blob: file(1) };
    const second = { kind: 'bend-right' as const, blob: file(2) };
    dropped.getState().dropFilms([first]);
    dropped.getState().dropFilms([second]);
    expect(dropped.getState().takeFilms()).toEqual([first, second]);
    expect(dropped.getState().films).toEqual([]);
    expect(dropped.getState().takeFilms()).toEqual([]);
  });

  it('replaces the films waiting, so the ones before are gone and the new ones are kept in order', () => {
    const dropped = createDropped();
    const old = { kind: 'front' as const, blob: file(1) };
    const next = [
      { kind: 'front' as const, blob: file(2) },
      { kind: 'side' as const, blob: file(3) },
    ];
    dropped.getState().dropFilms([old]);
    dropped.getState().replaceFilms(next);
    expect(dropped.getState().films).toEqual(next);
    expect(dropped.getState().takeFilms()).toEqual(next);
  });

  it('keeps one case file waiting, the newest, and takes it once', () => {
    const dropped = createDropped();
    expect(dropped.getState().takeCaseFile()).toBeNull();
    dropped.getState().dropCaseFile(file(1));
    const newest = file(2, 3);
    dropped.getState().dropCaseFile(newest);
    expect(dropped.getState().takeCaseFile()).toBe(newest);
    expect(dropped.getState().takeCaseFile()).toBeNull();
  });
});
