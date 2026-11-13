import { renamedFor } from '@/case-state/renamed-for';
import { filmOf, marksOf } from '@/case-state/selectors';
import type { CaseFileV1, FilmKind, FilmMarks, FilmToFit, FitFromFilms } from '@/shapes';
import type { FitSources } from '@/view-state/fit-sources';

type Openings = Readonly<Record<FilmKind, number>>;

class FitInputFailure extends Error {
  override name = 'FitInputFailure';
}

/** Effective geometry for this request only. The stored case is never edited. */
export function filmToFitFor(c: CaseFileV1, kind: FilmKind, sources: FitSources, openings: Openings): FilmToFit | null {
  const film = filmOf(c, kind);
  if (film === null) {
    if (kind === 'front') throw new FitInputFailure('fit-input-missing-front-film');
    return null;
  }
  const stored = marksOf(c, kind);
  if (stored === null && kind !== 'side') {
    if (kind === 'front') throw new FitInputFailure('fit-input-missing-front-marks');
    return null;
  }
  const original: FilmMarks = stored ?? {
    film: 'side', vertebrae: [], landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: null },
    namedFor: film.geometry.imageRight ?? 'anterior',
  };
  const source = sources[kind];
  const emptySide = kind === 'side' && original.vertebrae.length === 0;
  if (!emptySide && (source === undefined || source.opening !== openings[kind])) {
    throw new FitInputFailure(`fit-input-missing-source: ${kind}, opening ${String(openings[kind])}`);
  }
  const facing = film.geometry.imageRight ?? original.namedFor;
  const renamed = renamedFor(original, facing);
  // Renaming recomputes C7 from its corners. Keep the original landmark's
  // point and flags too, including its last-bit rounding.
  const marks = { ...renamed, landmarks: { ...renamed.landmarks, c7: original.landmarks.c7 } };
  return {
    marks,
    geometry: { ...film.geometry, imageRight: facing },
    size: [film.image.width, film.image.height],
    markSpread: !emptySide && kind === 'front' && source?.finder === 'trained'
      ? { kind: 'share-of-height', share: 0.11, sharedShare: 0.08 }
      : { kind: 'millimetres', sdMm: 1, sharedSdMm: 0 },
    facingAssumed: film.geometry.imageRight === null || !(kind === 'side' ? film.geometry.confirmed.facing : film.geometry.confirmed.leftRight),
  };
}

/** Bending films do not build the standing spine. */
export function fitInputFor(c: CaseFileV1, sources: FitSources, openings: Openings): FitFromFilms {
  const front = filmToFitFor(c, 'front', sources, openings);
  if (front === null) throw new FitInputFailure('fit-input-missing-front-film');
  return { front, side: filmToFitFor(c, 'side', sources, openings), typical: 'built-in', twistReadings: [] };
}

/** Only the standing request, its finding sources and its film openings. */
export function fitKeyFor(c: CaseFileV1, sources: FitSources, openings: Openings): string {
  return JSON.stringify({
    input: fitInputFor(c, sources, openings),
    sources: { front: sources.front ?? null, side: sources.side ?? null },
    openings: { front: openings.front, side: openings.side },
  });
}
