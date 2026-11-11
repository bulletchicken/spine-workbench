// The sample case as one StandInStory: the story the stand-in engine
// answers from in development, its default export found by this path.
// It is data the stand-in is handed; nothing here knows the stand-in.
//
// Films are painted the first time anything asks for their bytes,
// pixels or facts, so loading the story costs nothing until then. Every
// read hands out its own copy, so a reader may change what it is given.

import type { FilmKind, FilmMarks, LevelResidual, MadeUpSpine, TwoFilmCheck } from '@/shapes';
import { FILM_KINDS, MM_PER_PX } from './anatomy';
import { answerKey } from './answer-key';
import { pngFacts, pngOf, SAMPLE_FILMS } from './films';
import { guessFor, pngGuessFor } from './guesses';
import { keyframesOnly } from './keyframes-only';
import { passMarks, quadOf } from './marks';
import { refusedCase } from './refused-files';
import type { StandInStory } from './story-shape';

function storyFilm(kind: FilmKind): StandInStory['films'][FilmKind] {
  const film = SAMPLE_FILMS[kind];
  return {
    get file() {
      return film.file;
    },
    get facts() {
      return film.facts;
    },
    get thumbnail() {
      return film.thumbnail;
    },
    pixels: () => film.pixels(),
    get guess() {
      return guessFor(kind);
    },
  };
}

/** The front and side chains solved together: how far apart each level's height is on the two films, in mm. */
function twoFilmsOf(marks: FilmMarks[], mmPerPx: number): TwoFilmCheck {
  const front = marks.find((film) => film.film === 'front')?.vertebrae ?? [];
  const side = marks.find((film) => film.film === 'side')?.vertebrae ?? [];
  const heightOf = (vertebra: (typeof front)[number]) => {
    const [supA, supB, infB, infA] = quadOf(vertebra.marks);
    const edge = (a: typeof supA, b: typeof supA) => Math.hypot(b.at[0] - a.at[0], b.at[1] - a.at[1]);
    return ((edge(supA, infA) + edge(supB, infB)) / 2) * mmPerPx;
  };
  const residuals: LevelResidual[] = front.flatMap((vertebra) => {
    const partner = side.find((other) => other.level === vertebra.level);
    return partner === undefined ? [] : [{ level: vertebra.level, residual: Math.round(Math.abs(heightOf(vertebra) - heightOf(partner)) * 100) / 100 }];
  });
  return { certifiedGap: 0.6, exact: true, disagree: false, unit: 'mm', residuals };
}

const spine: MadeUpSpine = {
  get films() {
    return FILM_KINDS.map((kind) => ({ kind, png: pngOf(kind) }));
  },
  get answerKey() {
    return answerKey();
  },
};

function madeUpFilm(kind: FilmKind): StandInStory['madeUp']['films'][FilmKind] {
  return {
    get facts() {
      return pngFacts(kind);
    },
    get guess() {
      return pngGuessFor(kind);
    },
  };
}

const story: StandInStory = {
  films: { front: storyFilm('front'), side: storyFilm('side'), 'bend-left': storyFilm('bend-left'), 'bend-right': storyFilm('bend-right') },
  get pass() {
    const marks = passMarks();
    return {
      frames: keyframesOnly(marks),
      marks,
      twoFilms: twoFilmsOf(marks, MM_PER_PX),
      warnings: [],
    };
  },
  madeUp: {
    spine,
    films: { front: madeUpFilm('front'), side: madeUpFilm('side'), 'bend-left': madeUpFilm('bend-left'), 'bend-right': madeUpFilm('bend-right') },
  },
  get refusedCase() {
    return refusedCase();
  },
};

export default story;
