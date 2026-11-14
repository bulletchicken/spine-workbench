import type { AnchorChange, AnchorKind, BenchStop, CannotTellCode, Level, LookCode } from '@/shapes';

export const PLACE_WORDS = {
  step: 'Place', enter: 'Place screws and rods', back: 'Back', more: 'More', done: 'Done', next: 'Next', retry: 'Try again', backToThree: 'Back to 3D',
  working: 'Working out a first placement…', changed: 'The spine changed, so the placement was worked out again.',
  placed: (upper: Level, lower: Level) => `A first placement: ${upper} to ${lower}.`,
  sketch: 'A sketch from the ends of the main curve: no pair of ends from the rules could be placed on this spine.',
  sizing: "Sized from published tables, not from this spine's bone.",
  review: (count: number) => `Review the ${String(count)}`,
  progress: (at: number, count: number) => `${String(at)} of ${String(count)}`,
  counts: (screws: number, hooks: number, empty: number) => `${String(screws)} ${screws === 1 ? 'screw' : 'screws'}, ${String(hooks)} ${hooks === 1 ? 'hook' : 'hooks'}, ${String(empty)} left empty.`,
  everyPedicle: 'Every pedicle', levelNames: 'Level names',
  pedicleName: (level: Level, side: 'left' | 'right') => `${level}, ${side} of the front film`,
  endDrawn: (upper: boolean, level: Level) => `The ${upper ? 'upper' : 'lower'} end is drawn at ${level}.`,
  needsLook: 'What needs a look', cannotTell: 'What these films cannot tell', about: 'About these numbers',
  unsaved: 'The placement is not saved with the case yet.',
  facing: 'Left and right assume the front film is shown the usual way round.',
  names: 'The level names are estimates. If they are one off, this placement sits on the same bones under other names, and the tables then give other sizes at some places.',
  noPlacement: 'No placement could be worked out for this spine.',
  failed: 'The placement did not finish. Nothing was changed.',
  refused: 'A placement cannot be worked out for this spine as it stands.',
  origin: 'from tables for teenagers: no age was given',
  pedicle: (low: number, high: number) => `Pedicle ${(Math.floor(low * 10) / 10).toFixed(1)} to ${(Math.ceil(high * 10) / 10).toFixed(1)} mm, `,
  part: (level: Level, side: 'left' | 'right', kind: AnchorKind) => `${level}, ${side} of the front film · ${kind === 'none' ? 'empty place' : kind}`,
  rod: (side: 'left' | 'right') => `Rod, ${side} of the front film`,
  end: (upper: boolean, level: Level) => `${upper ? 'Upper' : 'Lower'} end · ${level}`,
  size: (diameter: number, length: number) => `${diameter.toFixed(1)} × ${String(Math.round(length * 10) / 10)} mm`,
  diameter: (value: number) => `${value.toFixed(1)} mm`,
  also: (widths: readonly number[], lengths: readonly number[]) => {
    const wide = widths.length === 0 ? '' : `${widths.map((v) => v.toFixed(1)).join(', ')} mm wide`;
    const long = lengths.length === 0 ? '' : `${lengths.map((v) => String(Math.round(v * 10) / 10)).join(', ')} mm long`;
    return wide === '' && long === '' ? '' : `Also ${[wide, long].filter(Boolean).join(' and ')} in some versions.`;
  },
  wider: (count: number, total: number) => `Wider than the pedicle in ${String(count)} of ${String(total)} versions.`,
  noCurve: 'No side-to-side curve was found.',
  addedLooks: (sentences: readonly string[]) => `Now: ${sentences.join(' ')}`,
  removedLooks: (sentences: readonly string[]) => `No longer: ${sentences.join(' ')}`,
  otherDiameters: 'Other published diameters',
  otherDiameter: (level: Level, side: 'left' | 'right', size: number, ratio: number) => `${level}, ${side} of the front film: ${size.toFixed(1)} mm, from a width ratio of ${String(ratio)}.`,
  endWithOther: (upper: boolean, drawn: Level, count: number, total: number, other: Level, otherCount: number) => `The ${upper ? 'upper' : 'lower'} end is drawn at ${drawn}: ${drawn} in ${String(count)} of ${String(total)} versions, ${other} in ${String(otherCount)}.`,
  endAlone: (upper: boolean, drawn: Level, count: number, total: number) => `The ${upper ? 'upper' : 'lower'} end is ${drawn} in only ${String(count)} of ${String(total)} versions.`,
  endUnchosen: (upper: boolean, drawn: Level, other: Level, count: number, total: number) => `The ${upper ? 'upper' : 'lower'} end is drawn at ${drawn}: ${drawn} in no version, ${other} in ${String(count)} of ${String(total)}.`,
  pair: (upper: Level, lower: Level, count: number, total: number) => `Together, ${upper} to ${lower} holds in ${String(count)} of ${String(total)} versions.`,
  namingSummary: (shift: number, count: number) => `${shift < 0 ? 'One higher' : 'One lower'}: ${String(count)} ${count === 1 ? 'place changes' : 'places change'}.`,
  namingChange: (change: AnchorChange, side: 'left' | 'right') => `${change.level}, ${side} of the front film → ${change.renamedAs}: ${change.kindFrom === 'none' ? 'empty place' : change.kindFrom}${change.diameterFromMm === null ? '' : `, ${change.diameterFromMm.toFixed(1)} mm`}${change.lengthFromMm === null ? '' : ` × ${String(Math.round(change.lengthFromMm * 10) / 10)} mm`} → ${change.kindTo === 'none' ? 'empty place' : change.kindTo}${change.diameterToMm === null ? '' : `, ${change.diameterToMm.toFixed(1)} mm`}${change.lengthToMm === null ? '' : ` × ${String(Math.round(change.lengthToMm * 10) / 10)} mm`}.`,
  stage: 'Placement preview. Up and Down walk the parts; Left and Right cross sides; Enter keeps a part open; Escape closes it.',
  gallery: 'Place', galleryState: 'A made-up first placement',
};

export const LOOK_WORDS: Record<LookCode, string> = {
  'concave-apex-needs-ct': 'In some versions of this spine this pedicle is on the inside of the curve, at or beside its apex, where pedicles are the narrowest. A CT would show its real size.',
  'may-be-too-narrow': 'This pedicle may be too narrow for a screw.',
  'near-the-aorta': 'At this level, published scans of right thoracic curves place the aorta close to the pedicle on this side.',
  'direction-unreadable': 'The direction of this pedicle cannot be read from these films.',
  'shorter-than-tables': 'The length worked out here is shorter than the shortest screw.',
  'no-table-for-this-age': 'The published tables do not give what is needed to size this pedicle.',
  'chord-from-nearest-age': 'The length is taken from the nearest age in the tables.',
};
export function lookWords(code: LookCode, kind: AnchorKind, length: number | null = null, shortest?: number): string {
  return code === 'shorter-than-tables' && kind === 'screw' && length !== null && length === shortest
    ? 'The length worked out here is shorter than the shortest screw, so the shortest is shown.' : LOOK_WORDS[code];
}
export const CANNOT_TELL_WORDS: Record<CannotTellCode, string> = {
  'ends-unsettled': 'Where the placement should end is not settled by these films.',
  'names-may-be-one-off': 'The level names are estimates and may be one off.',
  'no-sacrum': "The centre line from the sacrum cannot be drawn on this film, so the rules that use it cannot be read.",
  'no-bending-films': "Both bending films, showing each curve's ends, are needed to read whether the smaller curves are stiff.",
  'standing-bends': 'The published rules that read bending films were made on bends taken lying down. Bends taken standing read differently.',
  'twist-not-measured': 'Twist is not measured from these films.', 'no-side-film': 'Kyphosis needs a side film that shows the vertebrae it is read between.',
  'shoulders-outside-the-model': 'Shoulder balance is not part of this model.',
  'no-rotation-grade': 'The L3-or-L4 rule needs a rotation grade, and no published source turns degrees of twist into that grade.',
  'not-this-curve-type': 'Some rules are written for other curve types, or in some versions the curves fit none of the published types, so those rules do not apply.',
  'no-curve': 'Some rules look for a curve in a region where none was found.', 'no-level-meets-the-rule': 'For some rules no fitted vertebra meets the test, or a vertebra the rule needs is not on these films.',
  'no-pedicle-table': 'The tables have no pedicle sizes for some levels.',
  'no-film-scale': 'Distances in millimetres cannot be read from a film with no scale. Screw sizes still come from the tables.',
};

/** The model limits, kept with the placement and bench descriptions. */
export const PLACE_LIMITS = {
  preview: 'An education and research preview; a surgeon decides.',
  tables: 'Pedicle widths and screw sizes come from published population tables. A CT would show this pedicle’s real size.',
  bones: 'The displayed bones are one adult’s vertebrae shaped to the fit. Entry points follow tables and generic geometry, rather than landmarks measured on this bone.',
  stiffness: 'Films without a known applied load cannot show tissue stiffness. Forces depend on assumed stiffness.',
  rods: 'Rods flex in the model but never bend permanently, so shape changes and forces can be an upper end. Versions outside the stiff-rod method are still counted.',
  populations: 'Joint stiffness comes from adult cadavers and a published table. Strengths come from adult bone and young animals.',
  tablePopulations: 'The tables mainly studied Chinese, Japanese, Thai and Indian groups; no population adjustment is made.',
  loads: 'Daily loads leave out muscles and use adult measurements. Activities make the standing load larger; they do not reconstruct a person’s movement.',
  walking: 'A year of walking compares rod swings with a laboratory test level. It gives neither a lifetime nor a date. Fusion timing, growth and maturity are outside the model.',
  fit: 'Fit noise is drawn independently for each vertebra, which can overstate noise between neighbouring joints. Typical side curves and inferred twist are not sampled as measurements.',
  names: 'Level names can be one off. Alternative names carry no probability. Shoulders, body balance, missing sacrum geometry and an unavailable rotation grade cannot be recovered from the fitted spine.',
  comparison: 'Published shape comparisons do not establish forces in a person. This model has not been compared with patients. Its numerical checks concern the calculation alone.',
  outside: 'Outside the rods, joints keep their standing shape. Changes of an unfused curve and body rebalancing are absent; read shape changes only inside the instrumented levels.',
  heads: 'Tightened heads never slip in the model. Hooks tie translation in both directions. Head offset and rod overhang are not measured on these films.',
  history: 'The model leaves out acute screw–bone compliance, relaxation, growth-friendly eligibility and the body’s response to history or other diseases.',
  method: 'The calculation finds a local constrained state, which need not be unique. A single elastic step does not describe the whole nonlinear rod response.',
  frontOnly: 'With a front film only, forward lean, depth, side wedge and inferred twist come from a typical spine. A load calculation does not make them measurements of this spine.',
  noScale: 'With no film scale, distances and the load’s lever arms belong to a typical-sized spine.',
};

export const SHARED_LOOK_WORDS = {
  'direction-unreadable': 'The direction of the pedicles cannot be read from these films.',
  'may-be-too-narrow': 'Every pedicle here may be too narrow for a screw.',
  'no-table-for-this-age': 'The published tables do not give what is needed to size these pedicles.',
  'chord-from-nearest-age': 'Lengths are taken from the nearest age in the tables.',
  'shorter-than-tables': 'The lengths worked out here are shorter than the shortest screw.',
} satisfies Partial<Record<LookCode, string>>;

export const PLACE_LOAD_WORDS = {
  load: 'Put it under load', stop: 'Stop', stops: 'Load stop', allEstimates: 'All estimates',
  loading: (versions: number) => `Loading it on ${String(versions)} versions of this spine…`,
  stopped: 'Stopped. Nothing was kept.', failed: 'It did not finish. Nothing was changed.',
  refused: 'This placement cannot be put under load as it stands.',
  backToPlacement: 'Back to the placement',
  missingLevels: 'This placement cannot be put under load: the fit does not hold every vertebra from T1 to L5.',
  addFilm: 'Add a film that shows T1 to L5.', before: 'Before: the spine as fitted.', noLoad: 'No load yet.',
  loaded: (stop: string, range: string | null) => range === null ? `${stop}.` : `${stop}: the main curve is ${range}.`,
  anchorHelper: (range: string) => `Heaviest anchor: ${range} of its strength allowance.`,
  fallback: 'The spine drawn at Rods in is one of the versions, the one nearest the middle.',
  noSettled: 'No version settled, so the spine is drawn as fitted.',
  mainCurve: 'The model reads the main curve between its own ends, so it can differ from the measurement in 3D.',
  placedCurve: "The curve is placed by the placement's own reading, which can sit one level from the model's.",
  twistMissing: 'Twist at the apex needs twist measured on the films.',
  kyphosisMissing: 'Kyphosis needs a side film that shows the vertebrae it is read between.',
  typicalRod: "Its contour aims for a typical upper back: no side film measured this one's.",
  dailyNotYet: 'Loads in daily life are not worked out yet.',
  trunkNotBalanced: 'The model does not rebalance the trunk above and below the rods.',
  weight: 'Weight not given: 29 to 63 kg is assumed, the range in one published group.',
  steps: 'Steps a day not given: 8,000 to 16,000 is assumed, without a published source.',
  strengthAllowance: 'of its strength allowance.',
  allowance: "An anchor's strength allowance is its published strength, lowered for weaker bone in some versions.",
  pastContext: (anchor:boolean) => anchor ? 'Its allowance is a published test strength, lowered for weaker bone in some versions. The count is over versions of what is not known, not a verdict on this placement.' : 'Its level is a published test strength. The count is over versions of what is not known, not a verdict on this placement.',
  partAssumed: "Assumed: strengths from tables, lowered by the model's own allowances.",
  rangeHolds: 'Each range holds the middle eight in ten of the versions.',
  varies: (name: string) => `Varies most with: ${name}.`,
  unsettled: (n: number, total: number) => `${String(n)} of ${String(total)} versions did not settle, so every range here is less sure.`,
  outside: (count: string) => `In ${count}, the rods are too soft for this model's method, so the level above and the level below are assumed.`,
  past: (kind: 'anchor' | 'rod-span' | 'rod', count: string) => `${kind === 'anchor' ? 'Past its strength allowance' : kind === 'rod-span' ? "Past the rod’s yield" : 'Past the test-bench level'} in ${count}.`,
  denominator: (count: number, total: number, unsettled: number) => unsettled === 0 ? `${String(count)} of ${String(total)} versions` : `${String(count)} of the ${String(total - unsettled)} versions that settled`,
  range: (low: string, high: string, unit: string) => low === high ? `about ${low}${unit}` : `${low}${unit} to ${high}${unit}`,
  span: (from: Level, to: Level, side: 'left' | 'right') => `${from} to ${to}, ${side} of the front film · rod span`,
  anchorRange: (level: Level, side: 'left' | 'right') => `${level}, ${side} of the front film · strength allowance`,
  stepsYear: (low: string, high: string) => `About ${low} to ${high} million steps in the year.`,
  noAbove: 'Starts at T1: there is no level above to compare.',
};
export const LOAD_STOPS = { before: 'Before', 'rods-in': 'Rods in', standing: 'Standing', 'bending-forward': 'Bending forward', 'year-of-walking': 'A year of walking' } as const;
// Restore the daily stops here only when their model is ready to show.
export const SHOWN_LOAD_STOPS: readonly BenchStop[] = ['before', 'rods-in'];
export const DAILY_LOADS_WITHHELD = !SHOWN_LOAD_STOPS.includes('standing');
export const ESTIMATE_TAGS = { 'set-by-the-films': 'Set by the films', 'comparison-only': 'For comparing placements only', assumed: 'Assumed' } as const;
export const ESTIMATE_NAMES = {
  'main-cobb': 'Main curve in the model', kyphosis: 'Kyphosis', 'apical-twist': 'Twist at the apex', 'heaviest-anchor-force': "Heaviest anchor’s force", 'heaviest-anchor-demand': "Heaviest anchor’s share of its strength allowance", 'heaviest-anchor-over-median': 'Heaviest anchor against the middle of the anchor forces', 'heaviest-rod-use': "Heaviest rod span’s share of its yield", 'junction-above': 'Level above', 'junction-below': 'Level below', 'heaviest-rod-swing': 'Heaviest rod swing against the test-bench level',
} satisfies Record<import('@/shapes').Measure, string>;
export const UNKNOWN_NAMES = { geometry: 'the fit', 'stiffness-scale': 'stiffness', 'rib-cage': 'the rib cage', bone: 'bone', weight: 'weight', steps: 'steps a day', walking: 'how the walking is done', 'rod-yield': 'rod strength', preload: "how much the joints stiffen under the body’s weight", 'slice-offset': "how far in front of the spine the trunk’s weight acts", 'contour-knock-down': 'how much bending weakens the rod against repeated loads' } satisfies Record<import('@/shapes').SampledUnknown, string>;
