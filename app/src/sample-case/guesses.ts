// What sensing each sample film's geometry answers: the engine's proposals
// before anyone has said anything (GeometryGuess, never saved).
//
// - Front, side and bending left have a size from their files' pixel
//   spacing; bending right, a PNG, has none.
// - The side film faces right, read from the bulge of the upper back.
// - Bending left came in upside down and needs two quarter turns.
// - The front film's lead marker reads L, on the image right, so it
//   points to the patient's left being there, with medium certainty: one
//   marker and no file tag to agree with it. It proposes; nobody has
//   confirmed a side. Bending right shows the same marker further left;
//   on bending left it has slid off the film.

import type { FilmKind, GeometryGuess, LeadMarker, PxRect, ScaleResult } from '@/shapes';
import { leadMarkerAt, MM_PER_PX, VIEWS } from './anatomy';

/** The scale a DICOM pixel spacing with calibration type GEOMETRY gives. */
const SIZE_FROM_FILE: ScaleResult = {
  mmPerPx: MM_PER_PX,
  mmPerPxVertical: null,
  source: { source: 'file', rule: 'geometry' },
  plane: 'assumed-depth',
  relativeUncertainty: 0.03,
  weak: false,
};

/** The box of the lead letter L on a front-view film, in film pixels. */
function letterBounds(kind: FilmKind): PxRect {
  const at = leadMarkerAt(VIEWS[kind]);
  // The letter spans 13 pixels left of the ring's centre to 15 right, and 18 up and down.
  return { left: Math.floor(at.x - 13 - 0.5), top: Math.floor(at.y - 18 - 0.5), right: Math.ceil(at.x + 15 - 0.5), bottom: Math.ceil(at.y + 18 - 0.5) };
}

function leadMarker(kind: FilmKind, score: number): LeadMarker {
  return {
    letter: 'l',
    bounds: letterBounds(kind),
    score,
    margin: 0.41,
    mirrored: false,
    rotationDeg: 355,
    cutOut: false,
    material: 'lead',
  };
}

const UPRIGHT_AS_HUNG: GeometryGuess['upright'] = {
  quarterTurns: 0,
  certainty: 'high',
  evidence: [
    { cue: 'neck-and-head', strength: 0.86 },
    { cue: 'pelvis-mass', strength: 0.82 },
    { cue: 'lungs', strength: 0.7 },
  ],
};

const NOTHING_POINTS_EITHER_WAY: GeometryGuess['imageRight'] = {
  imageRight: null,
  certainty: 'none',
  fileSays: null,
  evidence: [],
  looksMirrored: false,
};

const MARKER_SAYS_LEFT_ON_THE_RIGHT: GeometryGuess['imageRight'] = {
  imageRight: 'patient-left',
  certainty: 'medium',
  fileSays: null,
  evidence: [{ cue: 'lead-marker', strength: 0.83 }],
  looksMirrored: false,
};

const GUESSES: Record<FilmKind, GeometryGuess> = {
  front: {
    upright: UPRIGHT_AS_HUNG,
    imageRight: MARKER_SAYS_LEFT_ON_THE_RIGHT,
    polarity: 'as-stored',
    scales: [SIZE_FROM_FILE],
    leadMarkers: [leadMarker('front', 0.82)],
    textHints: [],
    collimation: [],
  },
  side: {
    upright: UPRIGHT_AS_HUNG,
    imageRight: {
      imageRight: 'anterior',
      certainty: 'high',
      fileSays: null,
      evidence: [
        { cue: 'upper-back-bulge', strength: 0.88 },
        { cue: 'posterior-elements', strength: 0.74 },
      ],
      looksMirrored: false,
    },
    polarity: 'as-stored',
    scales: [SIZE_FROM_FILE],
    leadMarkers: [],
    textHints: [],
    collimation: [],
  },
  'bend-left': {
    upright: {
      quarterTurns: 2,
      certainty: 'high',
      evidence: [
        { cue: 'neck-and-head', strength: 0.86 },
        { cue: 'pelvis-mass', strength: 0.82 },
        { cue: 'lungs', strength: 0.7 },
      ],
    },
    imageRight: NOTHING_POINTS_EITHER_WAY,
    polarity: 'as-stored',
    scales: [SIZE_FROM_FILE],
    leadMarkers: [],
    textHints: [],
    collimation: [],
  },
  'bend-right': {
    upright: UPRIGHT_AS_HUNG,
    imageRight: MARKER_SAYS_LEFT_ON_THE_RIGHT,
    polarity: 'as-stored',
    scales: [],
    leadMarkers: [leadMarker('bend-right', 0.79)],
    textHints: [],
    collimation: [],
  },
};

/** What sensing a sample film answers, as a fresh copy. */
export function guessFor(kind: FilmKind): GeometryGuess {
  return structuredClone(GUESSES[kind]);
}

/** The same for the film's PNG, as a made-up spine hands it over: a PNG carries no size. */
export function pngGuessFor(kind: FilmKind): GeometryGuess {
  return { ...guessFor(kind), scales: [] };
}
