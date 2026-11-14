import type { AnchorProposal, BodySide, LayoutProposals, LookCode, RodProposal } from '@/shapes';
import { MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import { placingOf } from '@/spine-3d/placing/place-vertebra';
const codes: LookCode[] = ['concave-apex-needs-ct', 'may-be-too-narrow', 'near-the-aorta', 'direction-unreadable', 'shorter-than-tables', 'no-table-for-this-age', 'chord-from-nearest-age'];
/** Deliberately made-up values, so every part and reason can be inspected. */
export function placementFixture(): LayoutProposals {
  const vertebrae = MADE_UP_CASE.fit.vertebrae.filter((v) => ['T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1'].includes(v.level));
  const anchors = vertebrae.flatMap((v, index) => (['left', 'right'] as BodySide[]).map((side, si): AnchorProposal => {
    const number = index * 2 + si, kind = number === 1 ? 'hook' : number === 3 || number === 7 || number === 9 ? 'none' : 'screw';
    const size = { diameterMm: 4.5, diameterAlsoSeen: [5], lengthMm: 30, lengthAlsoSeen: [35], screwToPedicle: { p10: .6, p50: .7, p90: .9 }, medialAngle: { angleDeg: 12, source: 'made-up-direction' } };
    const code = codes[number % codes.length];
    const looks: LookCode[] = [...new Set<LookCode>(['direction-unreadable', ...(number < codes.length && code !== undefined ? [code] : [])])];
    return { level: v.level, side, kind, entryMm: [...placingOf(v.pose).toPatient([-15, side === 'left' ? 12 : -12, 0])], entryStatus: 'from-template', directionUnit: [1, side === 'left' ? -.2 : .2, 0], directionStatus: 'from-tables',
      medialAngle: kind === 'screw' ? size.medialAngle : null, medialAnglesBeside: [], convergenceDeg: { p10: 10, p50: 12, p90: 14 },
      diameterMm: kind === 'screw' ? size.diameterMm : null, diameterAlsoSeen: [5], diameterStatus: kind === 'screw' ? 'from-tables' : 'missing', widthRatio: .8, diametersBeside: [], screwToPedicle: kind === 'screw' ? size.screwToPedicle : null,
      chordMm: 36, chordShare: .8, lengthMm: kind === 'screw' ? size.lengthMm : null, lengthAlsoSeen: [35], lengthStatus: kind === 'screw' ? 'from-tables' : 'missing', usualLengthMm: [25, 45],
      pedicle: { outerWidthMm: { p10: 4.6, p50: 5.4, p90: 6.9 }, innerWidthMm: { p10: 1.8, p50: 2.6, p90: 3.5 }, versionsTooNarrow: 3, versionsInnerUnder1Mm: 1, curveSide: 'concave', status: 'from-tables', sources: ['made-up-width'] },
      widerThanPedicle: [{ diameterMm: 4.5, versions: 3 }, { diameterMm: 5, versions: 6 }], screwHere: size, looks,
      diameterSources: ['made-up-width'], lengthSources: ['made-up-chord'], directionSources: ['made-up-direction'], lookSources: looks.map((code) => ({ code, sources: ['made-up-width'] })), sources: ['made-up-width', 'made-up-chord', 'made-up-direction'] };
  }));
  const rods = (['left', 'right'] as BodySide[]).map((side): RodProposal => {
    const points = anchors.filter((a) => a.side === side && a.kind !== 'none').map((a): [number, number, number] => [a.entryMm[0] - 5, a.entryMm[1], a.entryMm[2]]);
    return { side, curveSide: side === 'left' ? 'concave' : 'outside', material: 'cobalt-chrome', diameterMm: 5.5, targetKyphosisDeg: 25, targetKyphosisStatus: 'from-tables', coronalLine: 'l5-vertical', overbendDeg: 0, headOffsetMm: 5, lengthMm: 280, geometryStatus: 'from-template', pointsMm: points, standingPointsMm: points, sources: ['made-up-rod'] };
  });
  return { tier: 'two-films', versions: 32, curves: [], classification: [], selective: [], rules: [], pairs: [{ upper: 'T4', lower: 'L1', versionsHolding: 14, rulesAt: [], rulesNear: [], rulesAgainst: [], stopShort: [], checks: [], freeSegmentsBelow: 4 }],
    layouts: [{ id: 'first', change: { kind: 'first' }, upper: 'T4', lower: 'L1', anchors, rods, releasedLevels: ['T4', 'T5'] }], firstIsSketch: false, upperEnds: [{ level: 'T4', versions: 19 }, { level: 'T3', versions: 13 }], lowerEnds: [{ level: 'L1', versions: 32 }],
    choices: { screwDiametersMm: [4.5, 5], screwLengthsMm: [30, 35], rodDiametersMm: [5.5], rodMaterials: ['cobalt-chrome'], rods: [{ material: 'cobalt-chrome', diameterMm: 5.5 }], sources: ['made-up-width'] },
    cannotTell: [{ code: 'ends-unsettled', levels: ['T4'] }, { code: 'names-may-be-one-off', levels: [] }, { code: 'no-bending-films', levels: [] }, { code: 'twist-not-measured', levels: ['T7'] }], namingScenarios: [-1, 1].map((shift) => ({ shift, changes: [{ level: 'T7', side: 'left', renamedAs: shift < 0 ? 'T6' : 'T8', kindFrom: 'screw', kindTo: 'screw', diameterFromMm: 4.5, diameterToMm: 5, lengthFromMm: 30, lengthToMm: 35, looksAdded: ['may-be-too-narrow'], looksRemoved: [] }] })) };
}
