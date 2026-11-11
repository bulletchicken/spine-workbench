// The StandInStory: everything the stand-in engine answers from. The
// sample case's story is one; the type sits here rather than in the
// stand-in so that the sample case never imports the stand-in.
//
// The stand-in knows a film by its bytes alone, as the engine judges a
// file: bytes equal to a story film's `file`, or to one of the made-up
// spine's PNGs, are that film; anything else is refused. Each byte
// buffer a story hands out is its own copy, so a caller may transfer it.

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
      file: ArrayBuffer; // what openFilm is handed: the sample case's DICOM files, and a PNG for bending right
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
  madeUp: {
    spine: MadeUpSpine; // what makeUpSpine answers: each film as a PNG, and the answer key
    // What openFilm and senseGeometry answer for each of those PNGs. Their
    // pictures are the films', so the thumbnail and pixels are the films'.
    films: Record<FilmKind, { facts: FilmFacts; guess: GeometryGuess }>;
  };
  refusedCase: { file: ArrayBuffer; error: EngineError };
}
