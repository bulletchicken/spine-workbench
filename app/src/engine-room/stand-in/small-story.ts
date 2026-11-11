// The stand-in's own story: four small drawn films of three lumbar
// vertebrae (L3, L4, L5), told until the sample case's story exists, and
// always in the stand-in's own tests. It follows every frozen rule a
// story must: ids count from 0 and are unique across the films, levels
// run top to bottom, pedicles only on the front film, `namedFor` is the
// film's guessed facing, and the pass's frames follow the frame contract
// in the order the engine sends them. Every byte buffer it hands out is a
// fresh copy.
//
// The made-up spine shows the same four films, as PNGs of their own (the
// same pixels under another title), so a stand-in can tell a made-up
// spine's film from the story's by its bytes.
//
// Each film is drawn the same way: a dark film, a lighter band of soft
// tissue down the middle, and three bright bodies 54 pixels tall with a
// 16-pixel disc between them, drifting a few pixels sideways from one to
// the next. Image right is the patient's left on the front and bending
// films, and the front of the patient on the side film.

import type {
  ChainBody,
  Confidence,
  EngineError,
  FilmAnswer,
  FilmFacts,
  FilmKind,
  FilmMarks,
  GeometryGuess,
  ImageRight,
  Landmarks,
  Level,
  LevelNaming,
  Mark,
  PassWarning,
  PreviewTile,
  Px,
  TrueMark,
  TrueMarks,
  TwoFilmCheck,
  VertebraOnFilm,
  VertebraPose,
  WatchFrame,
} from '@/shapes';
import { halfFloatBits } from '@/sample-case/files/half-float';
import { writePng } from '@/sample-case/files/png-file';
import type { StandInStory } from '@/sample-case/story-shape';
import { isInside, pointIn, type Quad } from './quads';

const KINDS: readonly FilmKind[] = ['front', 'side', 'bend-left', 'bend-right'];
const LEVELS: readonly Level[] = ['L3', 'L4', 'L5'];

/** How each film is drawn: its size and how far each body drifts sideways. */
const LAYOUT: Record<FilmKind, { width: number; height: number; drift: [number, number, number] }> = {
  front: { width: 120, height: 300, drift: [-6, 0, 6] },
  side: { width: 110, height: 300, drift: [4, 0, -4] },
  'bend-left': { width: 120, height: 290, drift: [-10, 0, 8] },
  'bend-right': { width: 116, height: 300, drift: [10, 0, -8] },
};
const FIRST_TOP = 40;
const BODY_HEIGHT = 54;
const BODY_STEP = 70;
const HALF_WIDTH = 26;
/** Endplate tilt: the a-side end of each body's upper endplate sits this much higher. */
const TILT: [number, number, number] = [2, 0, -2];

const FACING: Record<FilmKind, ImageRight> = { front: 'patient-left', side: 'anterior', 'bend-left': 'patient-left', 'bend-right': 'patient-left' };

const CONFIDENCE: Confidence[] = [
  { score: 0.94, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: { sup: 1.2, inf: 1.4 }, margin: 3.1 },
  { score: 0.76, band: 'medium', reasons: ['weak-gap'], calibrated: false, endplateSpreadPx: { sup: 2.1, inf: 2.4 }, margin: 1.2 },
  { score: 0.48, band: 'low', reasons: ['near-film-edge'], calibrated: false, endplateSpreadPx: { sup: 3.5, inf: 3.9 }, margin: 0.4 },
];

const NAMING: LevelNaming = {
  template: 'standard',
  chosenBy: 'engine',
  alternatives: [
    { template: 'standard', chance: 0.82 },
    { template: 'extra-lumbar', chance: 0.18 },
  ],
  fromTop: null,
  fromBottom: [...LEVELS],
  flags: ['top-anchor-weak'],
  l5ConfirmedOn: null,
};

const mark = (at: Px): Mark => ({ at, source: 'auto', pinned: false, inferred: false });

/** Body `index` on film `kind`, in quad order: upper a-side, upper b-side, lower b-side, lower a-side. */
function bodyQuad(kind: FilmKind, index: number): Quad {
  const { width, drift } = LAYOUT[kind];
  const centre = width / 2 + drift[index as 0 | 1 | 2];
  const top = FIRST_TOP + BODY_STEP * index;
  const bottom = top + BODY_HEIGHT;
  const tilt = TILT[index as 0 | 1 | 2];
  const aSide = centre + HALF_WIDTH;
  const bSide = centre - HALF_WIDTH;
  return [
    [aSide, top - tilt],
    [bSide, top + tilt],
    [bSide, bottom + tilt],
    [aSide, bottom - tilt],
  ];
}

function vertebraOn(kind: FilmKind, index: number, id: number): VertebraOnFilm {
  const quad = bodyQuad(kind, index);
  const [supA, supB, infB, infA] = quad.map(mark) as [Mark, Mark, Mark, Mark];
  const level = LEVELS[index] as Level;
  let confidence = CONFIDENCE[index] as Confidence;
  let marks: VertebraOnFilm['marks'];
  if (kind === 'front') {
    const hidden = index === 2;
    if (hidden) confidence = { ...confidence, reasons: [...confidence.reasons, 'pedicle-hidden'] };
    marks = {
      kind: 'front',
      points: {
        supL: supA,
        supR: supB,
        infR: infB,
        infL: infA,
        pedL: { seen: 'visible', at: pointIn(quad, 0.2, 0.3), source: 'auto', pinned: false, inferred: false, ringScore: 0.8 },
        pedR: hidden ? { seen: 'not-visible', source: 'auto' } : { seen: 'visible', at: pointIn(quad, 0.8, 0.3), source: 'auto', pinned: false, inferred: false, ringScore: 0.7 },
      },
    };
  } else if (kind === 'side') {
    marks = { kind: 'side', points: { supA, supP: supB, infP: infB, infA } };
  } else {
    marks = { kind: 'bending', lines: { sup: [supA, supB], inf: [infA, infB] } };
  }
  return { id, level, marks, confidence, outline: null };
}

function landmarksOn(kind: FilmKind): Landmarks {
  const centre = LAYOUT[kind].width / 2;
  if (kind === 'front') {
    return {
      c7: null,
      sacrumTop: { view: 'front', sacL: mark([centre + 34, 250]), sacR: mark([centre - 18, 250]) },
      hips: [
        { side: 'left', centre: mark([centre + 40, 285]), radiusPx: 14 },
        { side: 'right', centre: mark([centre - 40, 285]), radiusPx: 14 },
      ],
    };
  }
  if (kind === 'side') {
    return { c7: null, sacrumTop: { view: 'side', sacA: mark([centre + 16, 252]), sacP: mark([centre - 28, 246]) }, hips: [{ side: 'merged', centre: mark([centre + 10, 285]), radiusPx: 15 }] };
  }
  return { c7: null, sacrumTop: null, hips: [] };
}

function marksOn(kind: FilmKind): FilmMarks {
  const firstId = KINDS.indexOf(kind) * LEVELS.length;
  return {
    film: kind,
    vertebrae: LEVELS.map((_, index) => vertebraOn(kind, index, firstId + index)),
    landmarks: landmarksOn(kind),
    naming: NAMING,
    namedFor: FACING[kind],
  };
}

/** The film's stored 8-bit pixels. */
function drawFilm(kind: FilmKind): Uint8Array {
  const { width, height } = LAYOUT[kind];
  const bodies = LEVELS.map((_, index) => bodyQuad(kind, index));
  const pixels = new Uint8Array(width * height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      let value = Math.abs(x - width / 2) < 45 ? 70 : 25;
      if (bodies.some((quad) => isInside(quad, [x, y]))) value = 200;
      if (kind === 'front' && x >= width - 20 && x <= width - 8 && y >= 10 && y <= 26) value = 255;
      pixels[y * width + x] = value;
    }
  }
  return pixels;
}

/** Half the size, each pixel the mean of four: the film's thumbnail, under 256 pixels on its longest side. */
function halved(pixels: Uint8Array, width: number, height: number): { width: number; height: number; pixels: Uint8Array } {
  const half = { width: Math.floor(width / 2), height: Math.floor(height / 2) };
  const out = new Uint8Array(half.width * half.height);
  for (let y = 0; y < half.height; y += 1) {
    for (let x = 0; x < half.width; x += 1) {
      const at = (dx: number, dy: number) => pixels[(2 * y + dy) * width + 2 * x + dx] ?? 0;
      out[y * half.width + x] = Math.round((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1)) / 4);
    }
  }
  return { ...half, pixels: out };
}

function factsOf(kind: FilmKind): FilmFacts {
  const { width, height } = LAYOUT[kind];
  return {
    kind,
    width,
    height,
    bitsStored: 8,
    source: 'png',
    keptTags: null,
    values: { offset: 0, inverted: false, invertedByUser: false, modality: { kind: 'linear', slope: 1, intercept: 0 } },
    flags: [],
    defaultWindow: { kind: 'centre-width', centre: 128, width: 256, function: 'linear', from: 'percentile' },
    exifQuarterTurns: null,
  };
}

function guessOf(kind: FilmKind): GeometryGuess {
  const { width } = LAYOUT[kind];
  return {
    upright: { quarterTurns: 0, certainty: 'high', evidence: [{ cue: 'pelvis-mass', strength: 0.8 }] },
    imageRight: {
      imageRight: FACING[kind],
      certainty: 'medium',
      fileSays: null,
      evidence: [{ cue: kind === 'side' ? 'posterior-elements' : 'lead-marker', strength: 0.7 }],
      looksMirrored: false,
    },
    polarity: 'as-stored',
    scales: [],
    leadMarkers:
      kind === 'front'
        ? [{ letter: 'l', bounds: { left: width - 20, top: 10, right: width - 8, bottom: 26 }, score: 0.9, margin: 0.4, mirrored: false, rotationDeg: 0, cutOut: false, material: 'lead' }]
        : [],
    textHints: [],
    collimation: [],
  };
}

function tileOf(pixels: Uint8Array, width: number, height: number): PreviewTile {
  const across = Math.floor(width / 8);
  const down = Math.floor(height / 8);
  const data = new Uint8Array(across * down);
  for (let j = 0; j < down; j += 1) for (let i = 0; i < across; i += 1) data[j * across + i] = pixels[(j * 8 + 4) * width + i * 8 + 4] ?? 0;
  return { width: across, height: down, origin: [4, 4], stepAcross: [8, 0], stepDown: [0, 8], encoding: 'unit', data };
}

function floats(values: number[]): Uint8Array {
  const bytes = new Uint8Array(values.length * 4);
  const view = new DataView(bytes.buffer);
  values.forEach((value, index) => {
    view.setFloat32(index * 4, value, true);
  });
  return bytes;
}

const WARNINGS: PassWarning[] = [{ code: 'top-cropped', film: 'front', level: null, region: { left: 20, top: 0, right: 100, bottom: 30 } }];

/** A film pass's frames: evidence, the centre line and the unrolled strip. */
function filmPassFrames(kind: FilmKind, pixels: Uint8Array, marks: FilmMarks): WatchFrame[] {
  const { width, height, drift } = LAYOUT[kind];
  const centre = width / 2;
  const tile = tileOf(pixels, width, height);
  const centres: Px[] = [[centre + drift[0], 12], ...LEVELS.map((_, index): Px => [centre + drift[index as 0 | 1 | 2], FIRST_TOP + BODY_STEP * index + BODY_HEIGHT / 2]), [centre + drift[2], 262]];
  const film = kind;
  const frames: WatchFrame[] = [
    { kind: 'stage-started', film, stage: 'evidence' },
    { kind: 'evidence', film, stage: 'evidence', map: 'ridge', step: 1, steps: 2, final: false, scaleMm: 4, tile },
    { kind: 'evidence', film, stage: 'evidence', map: 'ridge', step: 2, steps: 2, final: true, scaleMm: 4, tile },
    { kind: 'stage-started', film, stage: 'centre-line' },
    { kind: 'end-zones', film, stage: 'centre-line', top: { left: centre - 30, top: 0, right: centre + 30, bottom: 30 }, bottom: { left: centre - 30, top: 240, right: centre + 30, bottom: 270 }, topCropped: true, bottomCropped: false },
    { kind: 'centre-line', film, stage: 'centre-line', points: centres.map(([x, y]): Px => [Math.round(x), Math.round(y)]), refined: false },
    { kind: 'centre-line', film, stage: 'centre-line', points: centres, refined: true },
    { kind: 'width', film, stage: 'centre-line', samples: marks.vertebrae.map((_, index) => {
      const quad = bodyQuad(kind, index);
      return { left: pointIn(quad, 1, 0.5), right: pointIn(quad, 0, 0.5), confidence: 0.8 };
    }) },
  ];
  for (const warning of WARNINGS.filter((each) => each.film === kind)) frames.push({ kind: 'warning', film, stage: 'centre-line', warning });
  const rows = centres.length;
  const columns = 32;
  frames.push(
    { kind: 'stage-started', film, stage: 'strip' },
    { kind: 'strip-shape', film, stage: 'strip', centres, normals: centres.map((): Px => [1, 0]), columnPx: 2, columns },
    {
      kind: 'strip-rows',
      film,
      stage: 'strip',
      firstRow: 0,
      rows,
      columns,
      image: Uint8Array.from({ length: rows * columns }, (_, cell) => (cell % columns < 6 || cell % columns > 25 ? 60 : 190)),
      valid: new Uint8Array(Math.ceil((rows * columns) / 8)).fill(0xff),
    },
    { kind: 'barcode', film, stage: 'strip', values: floats([0.2, 0.9, 0.85, 0.9, 0.3]), highPass: floats([-0.4, 0.3, 0.25, 0.3, -0.3]) },
  );
  return frames;
}

/** The chain pass's frames for one film: the sweep, the locked chain, its names and how sure each is. */
function chainFrames(kind: FilmKind, marks: FilmMarks, solvedWithSide: boolean): WatchFrame[] {
  const film = kind;
  const bodies: ChainBody[] = marks.vertebrae.map((_, index) => {
    const [supA, supB, infB, infA] = bodyQuad(kind, index);
    return { level: LEVELS[index] as Level, sup: { a: supA, b: supB }, inf: { a: infA, b: infB }, inferred: false };
  });
  const heat = (index: number) => Uint8Array.from({ length: 256 }, (_, bin) => Math.max(0, 255 - Math.abs(bin - (60 + 70 * index)) * 4));
  const frames: WatchFrame[] = [{ kind: 'stage-started', film, stage: 'chain' }];
  bodies.forEach((body, index) => {
    frames.push({ kind: 'sweep', film, stage: 'chain', level: body.level, heat: heat(index), tentative: bodies.slice(0, index + 1) });
  });
  frames.push(
    { kind: 'template-scored', film, stage: 'chain', template: 'standard', score: 0.8, chance: 0.82 },
    { kind: 'template-scored', film, stage: 'chain', template: 'extra-lumbar', score: 0.2, chance: 0.18 },
  );
  if (solvedWithSide) {
    frames.push(
      { kind: 'joint-progress', film, stage: 'chain', start: 0, round: 1, upper: 1.2, lower: 0.4 },
      { kind: 'joint-progress', film, stage: 'chain', start: 0, round: 2, upper: 0.9, lower: 0.6 },
    );
  }
  frames.push(
    { kind: 'chain-locked', film, stage: 'chain', bodies: bodies.slice().reverse() },
    { kind: 'stage-started', film, stage: 'naming' },
    { kind: 'levels-named', film, stage: 'naming', naming: marks.naming, levels: [...LEVELS] },
    { kind: 'stage-started', film, stage: 'confidence' },
    { kind: 'confidence', film, stage: 'confidence', levels: marks.vertebrae.map((vertebra) => ({ level: vertebra.level, score: vertebra.confidence.score, band: vertebra.confidence.band })) },
    { kind: 'outlines', film, stage: 'confidence', quads: marks.vertebrae.map((vertebra, index) => ({ level: vertebra.level, corners: bodyQuad(kind, index) })), final: true },
  );
  return frames;
}

const TWO_FILMS: TwoFilmCheck = {
  certifiedGap: 0.3,
  exact: true,
  disagree: false,
  unit: 'px',
  residuals: [
    { level: 'L3', residual: 0.6 },
    { level: 'L4', residual: 0.8 },
    { level: 'L5', residual: 1.1 },
  ],
};

/** Where the truth lies: three pixels right of and four below each mark, five pixels away. */
const TRUTH_OFFSET: Px = [3, 4];
const truly = (at: Px): TrueMark => ({ at: [at[0] + TRUTH_OFFSET[0], at[1] + TRUTH_OFFSET[1]], visible: true });

function trueMarksOf(vertebra: VertebraOnFilm, quad: Quad): TrueMarks {
  const [supA, supB, infB, infA] = quad.map(truly) as [TrueMark, TrueMark, TrueMark, TrueMark];
  switch (vertebra.marks.kind) {
    case 'front':
      return { kind: 'front', supL: supA, supR: supB, infR: infB, infL: infA, pedL: truly(pointIn(quad, 0.2, 0.3)), pedR: truly(pointIn(quad, 0.8, 0.3)) };
    case 'side':
      return { kind: 'side', supA, supP: supB, infP: infB, infA };
    case 'bending':
      return { kind: 'bending', sup: [supA, supB], inf: [infA, infB] };
  }
}

const POSES: VertebraPose[] = LEVELS.map((level, index) => ({
  level,
  centre: [0, 0, 27 + 35 * (2 - index)],
  rotation: [0, 0, 0],
  height: 27,
  wedgeFront: 0,
  wedgeSide: 0,
  topHalfWidth: 26,
  topHalfDepth: 20,
  bottomHalfWidth: 26,
  bottomHalfDepth: 20,
}));

function answerFor(kind: FilmKind, marks: FilmMarks): FilmAnswer {
  const centre = LAYOUT[kind].width / 2;
  return {
    kind,
    geometry: {
      mmPerPx: 0.5,
      mmPerPxVertical: null,
      scale: { source: 'file', rule: 'fiducial' },
      quarterTurns: 0,
      imageRight: FACING[kind],
      confirmed: { upright: true, leftRight: true, facing: true, scale: true, polarity: true, privacy: true },
    },
    vertebrae: marks.vertebrae.map((vertebra, index) => ({ level: vertebra.level, marks: trueMarksOf(vertebra, bodyQuad(kind, index)) })),
    landmarks: {
      c7: { at: [centre, -150], visible: false },
      sacrumTop: [truly([centre + 34, 250]), truly([centre + 8, 250]), truly([centre - 18, 250])],
      hips: [truly([centre + 40, 285]), truly([centre - 40, 285])],
    },
    bentPoses: kind === 'bend-left' || kind === 'bend-right' ? POSES : null,
  };
}

const REFUSED_CASE: EngineError = {
  code: 'case-unknown-field',
  message: 'case.json has a field this version does not know',
  detail: 'films[0].colour',
};

/** The small story, made on first use. */
export function smallStory(): StandInStory {
  const pixels = Object.fromEntries(KINDS.map((kind) => [kind, drawFilm(kind)])) as Record<FilmKind, Uint8Array>;
  const marks = KINDS.map(marksOn);
  const png = (kind: FilmKind, title: string) => {
    const { width, height } = LAYOUT[kind];
    return writePng(pixels[kind], width, height, { keyword: 'Title', value: title });
  };
  const files = Object.fromEntries(KINDS.map((kind) => [kind, png(kind, `small story: ${kind} film`)])) as Record<FilmKind, Uint8Array>;
  const madeUpPngs = Object.fromEntries(KINDS.map((kind) => [kind, png(kind, `small story: made-up ${kind} film`)])) as Record<FilmKind, Uint8Array>;

  const films = Object.fromEntries(
    KINDS.map((kind) => {
      const { width, height } = LAYOUT[kind];
      const file = files[kind];
      const half = halved(pixels[kind], width, height);
      const thumbnail = writePng(half.pixels, half.width, half.height);
      let display: Uint16Array | null = null;
      return [
        kind,
        {
          get file() {
            return file.slice().buffer;
          },
          facts: factsOf(kind),
          get thumbnail() {
            return thumbnail.slice();
          },
          pixels: () => (display ??= Uint16Array.from(pixels[kind], (value) => halfFloatBits(value / 255))).slice(),
          guess: guessOf(kind),
        },
      ];
    }),
  ) as StandInStory['films'];

  const frames = [
    ...KINDS.flatMap((kind, index) => filmPassFrames(kind, pixels[kind], marks[index] as FilmMarks)),
    ...KINDS.flatMap((kind, index) => chainFrames(kind, marks[index] as FilmMarks, kind === 'front')),
  ];
  const answerKey = {
    recipe: { name: 'straight', seed: 1, tier: 't0' },
    template: 'standard',
    poses: POSES,
    films: KINDS.map((kind, index) => answerFor(kind, marks[index] as FilmMarks)),
    angles: { frontalCobb: [{ region: 'thoracolumbar-lumbar', degrees: 0 }], apicalTwist: [], kyphosisDegrees: 0, lordosisDegrees: 0 },
  } satisfies StandInStory['madeUp']['spine']['answerKey'];

  return {
    films,
    pass: { frames, marks, twoFilms: TWO_FILMS, warnings: WARNINGS },
    madeUp: {
      spine: {
        get films() {
          return KINDS.map((kind) => ({ kind, png: madeUpPngs[kind].slice() }));
        },
        answerKey,
      },
      films: Object.fromEntries(KINDS.map((kind) => [kind, { facts: factsOf(kind), guess: guessOf(kind) }])) as StandInStory['madeUp']['films'],
    },
    get refusedCase() {
      return { file: new TextEncoder().encode('PK not a case this app can open').buffer, error: REFUSED_CASE };
    },
  };
}
