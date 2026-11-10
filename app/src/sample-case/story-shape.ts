// The StandInStory: everything the stand-in engine answers from. The
// sample case's story is one; the type sits here rather than in the
// stand-in so that the sample case never imports the stand-in.

import type {
  EngineError,
  FilmFacts,
  FilmKind,
  FilmMarks,
  GeometryGuess,
  MadeUpSpine,
  PassWarning,
  TwoFilmCheck,
  WatchFrame,
} from '@/shapes';

export interface StandInStory {
  films: Record<
    FilmKind,
    {
      file: ArrayBuffer; // what openFilm is handed: a PNG whose text chunk names this film
      facts: FilmFacts;
      thumbnail: Uint8Array;
      pixels(): Uint16Array; // half-float bits, facts.width × facts.height
      guess: GeometryGuess;
    }
  >;
  pass: {
    // ids count from 0; the stand-in adds the input's firstVertebraId
    frames: WatchFrame[]; // in the order the engine emits them
    marks: FilmMarks[];
    twoFilms: TwoFilmCheck | null;
    warnings: PassWarning[];
  };
  madeUp: MadeUpSpine;
  refusedCase: { file: ArrayBuffer; error: EngineError };
}
