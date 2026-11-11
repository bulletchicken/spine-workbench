import { inflateSync } from 'node:zlib';
import type { FilmKind, Stage } from '@/shapes';
import { LEVELS } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { c7Of, cornersOf } from './quads';
import { smallStory } from './small-story';

const story = smallStory();
const KINDS: FilmKind[] = ['front', 'side', 'bend-left', 'bend-right'];
const STAGES: Stage[] = ['evidence', 'centre-line', 'strip', 'chain', 'naming', 'confidence'];

/** A PNG's text chunk and its pixels, read the plain way (8-bit greyscale, filter 0 on every row). */
function readPng(file: ArrayBuffer | Uint8Array) {
  const png = file instanceof Uint8Array ? file : new Uint8Array(file);
  const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
  let text = '';
  let width = 0;
  let height = 0;
  let data = new Uint8Array(0);
  for (let at = 8; at < png.length; ) {
    const length = view.getUint32(at);
    const type = new TextDecoder().decode(png.subarray(at + 4, at + 8));
    const body = png.subarray(at + 8, at + 8 + length);
    if (type === 'IHDR') [width, height] = [view.getUint32(at + 8), view.getUint32(at + 12)];
    if (type === 'tEXt') text = new TextDecoder('latin1').decode(body);
    if (type === 'IDAT') data = new Uint8Array(inflateSync(body));
    at += 12 + length;
  }
  const pixels = new Uint8Array(width * height);
  for (let y = 0; y < height; y += 1) pixels.set(data.subarray(y * (width + 1) + 1, (y + 1) * (width + 1)), y * width);
  return { text, width, height, pixels };
}

describe('the small story', () => {
  it('gives each film a PNG that names it and holds its drawn pixels', () => {
    for (const kind of KINDS) {
      const film = story.films[kind];
      const png = readPng(film.file);
      expect(png.text).toBe(`Title\0small story: ${kind} film`);
      expect([png.width, png.height]).toEqual([film.facts.width, film.facts.height]);
      // The bare film at the corner is 25, the soft tissue down the middle 70.
      expect(png.pixels[0]).toBe(25);
      expect(png.pixels[2 * png.width + Math.round(png.width / 2)]).toBe(70);
      expect(film.pixels()).toHaveLength(png.width * png.height);
    }
  });

  it('gives each film a thumbnail at most 256 pixels on its longest side, half its size', () => {
    for (const kind of KINDS) {
      const thumbnail = readPng(story.films[kind].thumbnail);
      expect(Math.max(thumbnail.width, thumbnail.height)).toBeLessThanOrEqual(256);
      expect(thumbnail.width).toBe(Math.floor(story.films[kind].facts.width / 2));
    }
  });

  it('keeps vertebra ids unique across the films, counting from 0', () => {
    const ids = story.pass.marks.flatMap((marks) => marks.vertebrae.map((vertebra) => vertebra.id));
    expect(ids).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
  });

  it('runs every film’s levels top to bottom, L3 to L5, as both the marks and the counts say', () => {
    for (const marks of story.pass.marks) {
      expect(marks.vertebrae.map((vertebra) => vertebra.level)).toEqual(['L3', 'L4', 'L5']);
      expect(marks.naming.fromBottom).toEqual(['L3', 'L4', 'L5']);
      const order = marks.vertebrae.map((vertebra) => LEVELS.indexOf(vertebra.level));
      expect([...order].sort((a, b) => a - b)).toEqual(order);
    }
  });

  it('puts pedicles on the front film only, and each film’s marks are of its kind', () => {
    for (const marks of story.pass.marks) {
      const kinds = new Set(marks.vertebrae.map((vertebra) => vertebra.marks.kind));
      expect([...kinds]).toEqual([marks.film === 'front' ? 'front' : marks.film === 'side' ? 'side' : 'bending']);
    }
  });

  it('names the marks for the facing the film’s guess gives, and keeps C7 as the rule says', () => {
    for (const marks of story.pass.marks) {
      expect(marks.namedFor).toBe(story.films[marks.film].guess.imageRight.imageRight);
      expect(marks.landmarks.c7).toEqual(c7Of(marks.vertebrae));
      expect(marks.naming.l5ConfirmedOn).toBeNull();
    }
  });

  it('draws every outline as the quad its corners make, upper endplate above the lower', () => {
    for (const marks of story.pass.marks) {
      for (const vertebra of marks.vertebrae) {
        const [supA, supB, infB, infA] = cornersOf(vertebra);
        expect(supA.at[1]).toBeLessThan(infA.at[1]);
        expect(supB.at[1]).toBeLessThan(infB.at[1]);
      }
    }
  });

  it('sends each film’s frames stage by stage in the contract’s order, ending with final outlines', () => {
    for (const kind of KINDS) {
      const frames = story.pass.frames.filter((frame) => frame.film === kind);
      const stages = frames.map((frame) => STAGES.indexOf(frame.stage));
      expect([...stages].sort((a, b) => a - b)).toEqual(stages);
      expect(frames.filter((frame) => frame.kind === 'stage-started').map((frame) => frame.stage)).toEqual(STAGES);
      expect(frames.at(-1)).toMatchObject({ kind: 'outlines', final: true });
    }
  });

  it('runs every film pass before any chain frame, as the engine sends them', () => {
    const firstChain = story.pass.frames.findIndex((frame) => frame.stage === 'chain');
    const lastFilmPass = story.pass.frames.findLastIndex((frame) => ['evidence', 'centre-line', 'strip'].includes(frame.stage));
    expect(lastFilmPass).toBeLessThan(firstChain);
  });

  it('scores the made-up spine’s truth five pixels from every mark', () => {
    const front = story.madeUp.spine.answerKey.films[0];
    const marks = story.pass.marks[0];
    if (front?.vertebrae[0]?.marks.kind !== 'front' || marks?.vertebrae[0]?.marks.kind !== 'front') throw new Error('no front film');
    const truth = front.vertebrae[0].marks.supL.at;
    const placed = marks.vertebrae[0].marks.points.supL.at;
    expect(Math.hypot(truth[0] - placed[0], truth[1] - placed[1])).toBe(5);
  });

  it('locks each chain from the sacrum upward, the order the frozen frame gives', () => {
    for (const kind of KINDS) {
      const locked = story.pass.frames.find((frame) => frame.kind === 'chain-locked' && frame.film === kind);
      if (locked?.kind !== 'chain-locked') throw new Error(`no chain-locked frame on ${kind}`);
      expect(locked.bodies.map((body) => body.level)).toEqual(['L5', 'L4', 'L3']);
    }
  });

  it('gives the made-up spine each film as a PNG of its own: the same pixels under another title', () => {
    for (const { kind, png } of story.madeUp.spine.films) {
      const madeUp = readPng(png);
      const told = readPng(story.films[kind].file);
      expect(madeUp.text).toBe(`Title\0small story: made-up ${kind} film`);
      expect(madeUp.pixels).toEqual(told.pixels);
      expect(new Uint8Array(story.films[kind].file)).not.toEqual(png);
    }
  });

  it('hands out a fresh copy of every byte buffer, so a reader may transfer or change it', () => {
    const film = story.films.front;
    new Uint8Array(film.file).fill(0);
    film.thumbnail.fill(0);
    film.pixels().fill(0);
    story.madeUp.spine.films[0]?.png.fill(0);
    expect(new Uint8Array(film.file).subarray(0, 4)).toEqual(new Uint8Array([0x89, 0x50, 0x4e, 0x47]));
    expect(film.thumbnail.subarray(0, 4)).toEqual(new Uint8Array([0x89, 0x50, 0x4e, 0x47]));
    expect(film.pixels().some((bits) => bits !== 0)).toBe(true);
    expect(story.madeUp.spine.films[0]?.png.subarray(0, 4)).toEqual(new Uint8Array([0x89, 0x50, 0x4e, 0x47]));
  });
});

