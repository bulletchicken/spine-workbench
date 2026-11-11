// How a stand-in film worker answers each film job from the story. The
// worker holds one story film at a time, found by the bytes it was handed:
// a story film's file, or one of the made-up spine's PNGs, which shows the
// same film and answers with the made-up spine's facts and guess. Anything
// else is refused as the engine refuses a file it cannot read, judged by
// the bytes alone. A film that cannot be read leaves the open one as it
// was.
//
// The pixels, thumbnail and saved PNG are always those of the film the
// bytes show. The guess, the pass's frames and the marks follow that story
// film while its marks suit the slot it was opened into (a bending film in
// either bending slot); a film of another view answers as the slot's own
// story film, so every answer is one the slot can hold, as the engine's
// are. A film pass's strip names the story film it answered as, so the
// chain pass on the spine worker can follow it too.

import type { StandInStory } from '@/sample-case/story-shape';
import type { Film, FilmFacts, FilmKind, FilmMarks, ScaleResult, Stage } from '@/shapes';
import { EngineFailure } from '../engine-client';
import { boundaryWand } from './boundary-wand';
import { resolvePins } from './pins';
import { distance } from './quads';
import { regionWand } from './region-wand';
import type { StandInScripts } from './stand-in-engine';
import type { StandInAnswer, StandInJobs } from './stand-in-worker';
import { isBending, type ToolFilm } from './tool-film';
import { previewTool } from './tool-preview';

/**
 * The film a stand-in film worker holds: its slot, the story film its
 * bytes show, the story film it answers as, whether it came as a made-up
 * spine's PNG, and its facts.
 */
export interface FilmRoom {
  film: { kind: FilmKind; shows: FilmKind; story: FilmKind; madeUp: boolean; facts: FilmFacts } | null;
}

export const FILM_STAGES: readonly Stage[] = ['evidence', 'centre-line', 'strip'];

const KINDS: readonly FilmKind[] = ['front', 'side', 'bend-left', 'bend-right'];

/** The strip a stand-in film pass answers: the name of the story film it ran on. */
export function stripNaming(kind: FilmKind): Uint8Array {
  return new TextEncoder().encode(kind);
}

/** The story film a stand-in strip names, or null for any other strip. */
export function storyFilmOf(strip: Uint8Array): FilmKind | null {
  const named = new TextDecoder().decode(strip);
  return KINDS.find((kind) => kind === named) ?? null;
}

export function filmJobs(story: StandInStory, scripts: StandInScripts): StandInJobs<FilmRoom> {
  const held = (room: FilmRoom) => {
    if (room.film === null) throw new EngineFailure({ code: 'engine-out-of-sync', message: 'no film is open in this worker', detail: null });
    return room.film;
  };
  const answer = <Output>(delayMs: number, value: () => Output): StandInAnswer<Output> => ({ kind: 'answer', delayMs, answer: value });
  const tool = (room: FilmRoom): ToolFilm => held(room);
  const { films, sizeAndFacing, find, adjust, save } = scripts;

  /** The story film these bytes are, and whether they are the made-up spine's PNG of it. */
  const recognise = (bytes: Uint8Array): { story: FilmKind; madeUp: boolean } | null => {
    const film = KINDS.find((kind) => sameBytes(new Uint8Array(story.films[kind].file), bytes));
    if (film !== undefined) return { story: film, madeUp: false };
    const png = story.madeUp.spine.films.find((each) => sameBytes(each.png, bytes));
    return png === undefined ? null : { story: png.kind, madeUp: true };
  };

  return {
    'open-film': (input, room) =>
      answer(films.openMs, () => {
        const found = recognise(new Uint8Array(input.bytes));
        if (found === null) throw new EngineFailure({ code: 'film-unknown-format', message: 'these bytes are not one of the story’s films', detail: null });
        const told = found.madeUp ? story.madeUp.films[found.story].facts : story.films[found.story].facts;
        const facts = openedFacts(input.kind, told, input.saved);
        const answersAs = sameMarks(found.story, input.kind) ? found.story : input.kind;
        room.film = { kind: input.kind, shows: found.story, story: answersAs, madeUp: found.madeUp, facts };
        return { facts, thumbnail: story.films[found.story].thumbnail.slice() };
      }),
    'display-pixels': (_input, room) => answer(films.displayPixelsMs, () => story.films[held(room).shows].pixels().slice()),
    'encode-png': (_input, room) =>
      answer(save.encodeMs, () => {
        const { shows } = held(room);
        const png = story.madeUp.spine.films.find((each) => each.kind === shows)?.png;
        if (png === undefined) throw new EngineFailure({ code: 'engine-out-of-sync', message: 'the story has no PNG of this film', detail: shows });
        return png.slice();
      }),
    'sense-geometry': (_input, room) =>
      answer(sizeAndFacing.senseMs, () => {
        const film = held(room);
        return film.madeUp ? story.madeUp.films[film.story].guess : story.films[film.story].guess;
      }),
    'scale-from-ruler': ({ a, b, mm }, room) => {
      return answer(sizeAndFacing.scaleMs, () => {
        held(room);
        const length = distance(a, b);
        checkLength(length, mm, sizeAndFacing.shortestRulerPx);
        return {
          mmPerPx: mm / length,
          mmPerPxVertical: null,
          source: { source: 'ruler', rulers: [{ a, b, mm }] },
          plane: 'unknown',
          relativeUncertainty: (sizeAndFacing.clickErrorPx * Math.SQRT2) / length,
          weak: length < sizeAndFacing.weakRulerBelowPx,
        } satisfies ScaleResult;
      });
    },
    'scale-from-known-object': ({ near, edge, diameterMm }, room) => {
      return answer(sizeAndFacing.scaleMs, () => {
        held(room);
        const radiusPx = distance(near, edge);
        checkLength(2 * radiusPx, diameterMm, sizeAndFacing.shortestRulerPx);
        return {
          mmPerPx: diameterMm / (2 * radiusPx),
          mmPerPxVertical: null,
          source: { source: 'known-object', centre: near, radiusPx, diameterMm },
          plane: 'object-depth',
          relativeUncertainty: sizeAndFacing.clickErrorPx / radiusPx,
          weak: 2 * radiusPx < sizeAndFacing.weakRulerBelowPx,
        } satisfies ScaleResult;
      });
    },
    'film-pass': ({ film }, room) => {
      const { story: told } = held(room);
      const { frames: all, warnings } = story.pass;
      const frames = all.filter((frame) => frame.film === told && FILM_STAGES.includes(frame.stage)).map((frame) => ({ ...frame, film: film.kind }));
      const total = pace(scripts) * (1 - find.chainShare);
      return {
        kind: 'pass',
        frames,
        gapMs: total / Math.max(1, frames.length),
        answer: () => ({
          film: film.kind,
          strip: stripNaming(told),
          scaleAssumed: film.geometry.mmPerPx === null,
          warnings: warnings.filter((warning) => warning.film === told).map((warning) => ({ ...warning, film: film.kind })),
        }),
      };
    },
    'place-points': ({ chain, firstVertebraId }, room) => {
      return answer(find.placePointsMs, () => {
        const { story: told } = held(room);
        const marks = story.pass.marks.find((each) => each.film === told);
        if (marks === undefined) throw new EngineFailure({ code: 'pass-no-spine-found', message: 'the story has no marks for this film', detail: told });
        return renumbered({ ...marks, film: chain.film }, firstVertebraId);
      });
    },
    'tool-preview': (input, room) => answer(adjust.previewMs, () => previewTool(tool(room), input, adjust)),
    'region-wand': (input, room) => answer(adjust.wandMs, () => regionWand(tool(room), input, adjust)),
    'boundary-wand': (input, room) => answer(adjust.wandMs, () => boundaryWand(tool(room), input, adjust)),
    'resolve-pins': (input, room) => answer(adjust.pinMs, () => resolvePins(tool(room), input, adjust)),
  };
}

/** The whole pass at the pace the stand-in was started with. */
export function pace(scripts: StandInScripts): number {
  return scripts.pace === 'slow' ? scripts.find.slowMs : scripts.find.normalMs;
}

/** The facts of a film opened into slot `kind`; a case PNG takes its record's source, tags, values and flags. */
function openedFacts(kind: FilmKind, told: FilmFacts, saved: Film | null): FilmFacts {
  if (saved === null) return { ...told, kind };
  if (saved.image.width !== told.width || saved.image.height !== told.height) {
    throw new EngineFailure({ code: 'film-size-mismatch', message: 'the PNG does not match its saved record', detail: `${String(told.width)}×${String(told.height)}` });
  }
  return { ...told, kind, source: saved.source, keptTags: saved.keptTags, values: saved.values, flags: saved.flags };
}

function checkLength(lengthPx: number, mm: number, shortestPx: number): void {
  if (!(mm > 0) || !Number.isFinite(mm)) throw new EngineFailure({ code: 'scale-bad-length', message: 'a length must be more than zero', detail: String(mm) });
  if (lengthPx < shortestPx) throw new EngineFailure({ code: 'scale-too-short', message: 'too short to measure a scale from', detail: `${lengthPx.toFixed(1)} px` });
}

/** The story's marks with ids counting up from `first` in their order. */
function renumbered(marks: FilmMarks, first: number): FilmMarks {
  return { ...marks, vertebrae: marks.vertebrae.map((vertebra, index) => ({ ...vertebra, id: first + index })), naming: { ...marks.naming, l5ConfirmedOn: null } };
}

/** True when a film of kind `a` takes the same kind of marks as slot `b`: front, side, or either bending film. */
function sameMarks(a: FilmKind, b: FilmKind): boolean {
  return a === b || (isBending(a) && isBending(b));
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += 1) if (a[i] !== b[i]) return false;
  return true;
}
