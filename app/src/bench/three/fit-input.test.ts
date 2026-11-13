import { describe, expect, it } from 'vitest';
import { createCaseStore } from '@/case-state/case-store';
import { emptyCase } from '@/case-state/empty-case';
import { createFilmPixels } from '@/case-state/film-pixels';
import { renamedFor } from '@/case-state/renamed-for';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { filmRecordFrom } from '@/open-and-save/film-record';
import type { FilmKind, FrontFinder, ImageRight } from '@/shapes';
import { createFitSources } from '@/view-state/fit-sources';
import { filmToFitFor, fitInputFor, fitKeyFor } from './fit-input';

function visit(finder: FrontFinder = 'trained') {
  const story = smallStory();
  const store = createCaseStore({ ...emptyCase(), films: Object.values(story.films).map((film) => filmRecordFrom(film.facts)), marks: structuredClone(story.pass.marks), nextVertebraId: 12 });
  const sources = createFitSources(); const pixels = createFilmPixels();
  for (const film of store.getState().case.films) sources.getState().kept(film.kind, film.kind === 'front' ? finder : 'hand-built', 0);
  return { store, sources, pixels };
}

describe('standing fit input', () => {
  it('copies original marks, C7 and flags and supplies only transient facing', () => {
    const { store, sources, pixels } = visit();
    const c = store.getState().case;
    const original = c.marks[0]; if (original === undefined) throw new Error('no Front');
    const first = original.vertebrae[0]; if (first === undefined || first.marks.kind !== 'front') throw new Error('no Front body');
    first.level = 'C7'; first.marks.points.supL.inferred = true; first.marks.points.supL.pinned = true;
    original.landmarks.c7 = { at: [3, 7], source: 'hand', pinned: true, inferred: true };
    original.naming.flags = ['estimated'];
    const before = structuredClone(c); const version = store.getState().version; const history = store.getState().history;
    const input = fitInputFor(c, sources.getState().films, pixels.getState().openings);
    expect(input).toEqual({ front: { marks: original, geometry: { ...c.films[0]?.geometry, imageRight: original.namedFor }, size: [c.films[0]?.image.width, c.films[0]?.image.height], markSpread: { kind: 'share-of-height', share: 0.11, sharedShare: 0.08 }, facingAssumed: true }, side: filmToFitFor(c, 'side', sources.getState().films, pixels.getState().openings), typical: 'built-in', twistReadings: [] });
    expect(input.side?.markSpread).toEqual({ kind: 'millimetres', sdMm: 1, sharedSdMm: 0 });
    expect(input.front.geometry.mmPerPx).toBeNull(); expect(input.front.geometry.mmPerPxVertical).toBeNull();
    expect(input.front.marks.vertebrae[0]?.level).toBe('C7');
    expect(c).toEqual(before); expect(store.getState()).toMatchObject({ version, history });
    const film = c.films[0]; if (film === undefined) throw new Error('no Front');
    film.geometry.imageRight = original.namedFor === 'patient-left' ? 'patient-right' : 'patient-left';
    const renamed = filmToFitFor(c, 'front', sources.getState().films, pixels.getState().openings);
    expect(renamed?.marks.landmarks.c7).toEqual(original.landmarks.c7);
    const points = renamed?.marks.vertebrae[0]?.marks;
    if (points?.kind !== 'front') throw new Error('no renamed Front');
    expect(points.points.supR).toEqual(first.marks.points.supL);
  });

  it('uses the view’s own facing confirmation independently of scale and the other confirmation bit', () => {
    const { store, sources, pixels } = visit(); const c = store.getState().case;
    for (const film of c.films) {
      film.geometry.imageRight = film.kind === 'side' ? 'anterior' : 'patient-left';
      film.geometry.confirmed.leftRight = film.kind !== 'side';
      film.geometry.confirmed.facing = film.kind === 'side';
    }
    const input = fitInputFor(c, sources.getState().films, pixels.getState().openings);
    expect(input.front.facingAssumed).toBe(false); expect(input.side?.facingAssumed).toBe(false);
    expect(input.front.geometry.mmPerPx).toBeNull(); expect(input.side?.geometry.mmPerPx).toBeNull();
  });

  it.each([['front', 'patient-left', 'patient-right'], ['side', 'anterior', 'posterior'], ['bend-left', 'patient-left', 'patient-right']] as const)('uses both naming conventions and confirmed facing on %s', (kind, usual, reversed) => {
    const { store, sources, pixels } = visit('hand-built'); const c = store.getState().case;
    const film = c.films.find((f) => f.kind === kind); const marks = c.marks.find((m) => m.film === kind);
    if (film === undefined || marks === undefined) throw new Error('no film');
    for (const namedFor of [usual, reversed]) {
      marks.namedFor = namedFor;
      for (const saved of [null, usual, reversed]) for (const confirmed of [false, true]) {
        film.geometry = { ...film.geometry, quarterTurns: 1, mmPerPx: 0.4, mmPerPxVertical: 0.8, imageRight: saved, confirmed: { ...film.geometry.confirmed, facing: confirmed, leftRight: confirmed } };
        const before = structuredClone(c);
        const input = filmToFitFor(c, kind, sources.getState().films, pixels.getState().openings);
        const facing: ImageRight = saved ?? namedFor;
        expect(input?.geometry).toEqual({ ...film.geometry, imageRight: facing });
        expect(input?.marks).toEqual({ ...renamedFor(marks, facing), landmarks: { ...renamedFor(marks, facing).landmarks, c7: marks.landmarks.c7 } });
        expect(input?.facingAssumed).toBe(saved === null || !confirmed);
        expect(input?.markSpread).toEqual({ kind: 'millimetres', sdMm: 1, sharedSdMm: 0 });
        expect(c).toEqual(before);
      }
    }
  });

  it('uses provenance independently of estimated names and mark source', () => {
    const { store, sources, pixels } = visit(); const c = store.getState().case;
    for (const marks of c.marks) marks.naming.flags = ['estimated'];
    const input = () => fitInputFor(c, sources.getState().films, pixels.getState().openings);
    expect(input().front.markSpread.kind).toBe('share-of-height');
    const front = c.marks[0]; if (front === undefined) throw new Error('no Front');
    front.naming.flags = []; front.naming.chosenBy = 'user';
    expect(input().front.markSpread.kind).toBe('share-of-height');
    sources.getState().kept('front', 'hand-built', 0);
    expect(input().front.markSpread).toEqual({ kind: 'millimetres', sdMm: 1, sharedSdMm: 0 });
  });

  it('keeps sparse Side, supplies an empty Side, and refuses unknown marked provenance or missing Front', () => {
    const { store, sources, pixels } = visit(); const c = store.getState().case;
    const input = () => fitInputFor(c, sources.getState().films, pixels.getState().openings);
    const side = c.marks.find((m) => m.film === 'side'); if (side === undefined) throw new Error('no Side');
    side.vertebrae = side.vertebrae.slice(0, 1);
    expect(input().side?.marks.vertebrae).toHaveLength(1);
    sources.getState().forget('side'); expect(input).toThrow('fit-input-missing-source: side, opening 0');
    c.marks = c.marks.filter((m) => m.film !== 'side');
    expect(input().side).toMatchObject({ marks: { film: 'side', vertebrae: [], landmarks: { c7: null, sacrumTop: null, hips: [] }, naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: null }, namedFor: 'anterior' }, markSpread: { kind: 'millimetres', sdMm: 1, sharedSdMm: 0 }, facingAssumed: true });
    c.films = c.films.filter((f) => f.kind !== 'side'); expect(input().side).toBeNull();
    pixels.getState().opened('front', { pixels: new Uint16Array(), thumbnail: new Uint8Array() });
    expect(input).toThrow('fit-input-missing-source: front, opening 1');
    c.marks = []; expect(input).toThrow('fit-input-missing-front-marks');
    c.films = []; expect(input).toThrow('fit-input-missing-front-film');
  });

  it('keys actual standing input, sources and openings, excluding bend edits and review state', () => {
    const { store, sources, pixels } = visit(); const c = store.getState().case;
    const key = () => fitKeyFor(c, sources.getState().films, pixels.getState().openings);
    const was = key();
    c.savedWith.app = 'different'; c.nextVertebraId += 1;
    c.marks = c.marks.filter((m) => m.film !== 'bend-right');
    sources.getState().forget('bend-right');
    pixels.getState().opened('bend-right', { pixels: new Uint16Array(), thumbnail: new Uint8Array() });
    expect(key()).toBe(was);
    sources.getState().kept('front', 'hand-built', 0); expect(key()).not.toBe(was);
    const hand = key(); const front = c.films[0]; if (front === undefined) throw new Error('no Front');
    front.geometry.confirmed.leftRight = true; front.geometry.imageRight = 'patient-left'; expect(key()).not.toBe(hand);
    const confirmed = key();
    for (const kind of ['front', 'side'] satisfies FilmKind[]) {
      pixels.getState().opened(kind, { pixels: new Uint16Array(), thumbnail: new Uint8Array() });
      sources.getState().kept(kind, 'hand-built', pixels.getState().openings[kind]);
    }
    expect(key()).not.toBe(confirmed);
  });
});
