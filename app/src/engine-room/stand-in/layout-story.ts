// A fixed, made-up placement for the stand-in; it does not size a spine.

import type { AnchorProposal, BodySide, ImplantLayout, Level, RodProposal } from '@/shapes';

function anchor(level: Level, side: BodySide, z: number): AnchorProposal {
  return {
    level,
    side,
    kind: 'screw',
    entryMm: [-20, side === 'left' ? 12 : -12, z],
    entryStatus: 'from-template',
    directionUnit: [1, 0, 0],
    directionStatus: 'entered',
    medialAngle: null,
    medialAnglesBeside: [],
    convergenceDeg: { p10: 0, p50: 0, p90: 0 },
    diameterMm: 4.5,
    diameterAlsoSeen: [],
    diameterStatus: 'entered',
    widthRatio: null,
    diametersBeside: [],
    screwToPedicle: null,
    chordMm: null,
    chordShare: null,
    lengthMm: 30,
    lengthAlsoSeen: [],
    lengthStatus: 'entered',
    usualLengthMm: null,
    pedicle: {
      outerWidthMm: null,
      innerWidthMm: null,
      versionsTooNarrow: 0,
      versionsInnerUnder1Mm: 0,
      curveSide: 'outside',
      status: 'missing',
      sources: [],
    },
    looks: [],
    sources: [],
    widerThanPedicle: [],
    screwHere: null,
    diameterSources: [],
    lengthSources: [],
    directionSources: [],
    lookSources: [],
  };
}

function rod(side: BodySide): RodProposal {
  const y = side === 'left' ? 12 : -12;
  return {
    side,
    curveSide: 'outside',
    material: 'cobalt-chrome',
    diameterMm: 5.5,
    targetKyphosisDeg: 0,
    targetKyphosisStatus: 'entered',
    coronalLine: 'l5-vertical',
    overbendDeg: 0,
    headOffsetMm: 15,
    lengthMm: 30,
    geometryStatus: 'from-template',
    pointsMm: [[-35, y, 60], [-35, y, 30]],
    standingPointsMm: [],
    sources: [],
  };
}

export function sampleLayout(): ImplantLayout {
  return {
    id: 'changed-by-hand',
    change: { kind: 'changed-by-hand' },
    upper: 'T8',
    lower: 'T9',
    anchors: [anchor('T8', 'left', 60), anchor('T8', 'right', 60), anchor('T9', 'left', 30), anchor('T9', 'right', 30)],
    rods: [rod('left'), rod('right')],
    releasedLevels: ['T8'],
  };
}
