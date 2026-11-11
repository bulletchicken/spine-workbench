import { caseAt, findingsAt, STORY_STAGES } from '@/sample-case/stages';
import type { CaseFileV1, Confirmations, Film, FilmKind, FilmMarks, Level, Mark, VertebraOnFilm } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { checkCase } from './checks';

const mark: Mark = { at: [0, 0], source: 'auto', pinned: false, inferred: false };
const ALL_CONFIRMED: Confirmations = { upright: true, leftRight: true, facing: true, scale: true, polarity: true, privacy: true };
const STANDARD: Level[] = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];

function vertebra(id: number, level: Level, kind: FilmKind, extra: Partial<VertebraOnFilm> = {}): VertebraOnFilm {
  const confidence = { score: 0.9, band: 'high' as const, reasons: [], calibrated: false, endplateSpreadPx: null, margin: null };
  const marks: VertebraOnFilm['marks'] =
    kind === 'front'
      ? { kind: 'front', points: { supL: mark, supR: mark, infR: mark, infL: mark, pedL: { seen: 'visible', at: [0, 0], source: 'auto', pinned: false, inferred: false, ringScore: null }, pedR: { seen: 'visible', at: [0, 0], source: 'auto', pinned: false, inferred: false, ringScore: null } } }
      : { kind: 'side', points: { supA: mark, supP: mark, infP: mark, infA: mark } };
  return { id, level, marks, confidence, outline: null, ...extra };
}

function film(kind: FilmKind, confirmed: Partial<Confirmations> = {}): Film {
  return {
    kind,
    image: { path: `films/${kind}.png`, width: 10, height: 10, bitsStored: 8 },
    source: 'png',
    keptTags: null,
    values: { offset: 0, inverted: false, invertedByUser: false, modality: { kind: 'linear', slope: 1, intercept: 0 } },
    flags: [],
    blanked: [],
    geometry: { mmPerPx: 0.2, mmPerPxVertical: null, scale: null, quarterTurns: 0, imageRight: kind === 'side' ? 'anterior' : 'patient-left', confirmed: { ...ALL_CONFIRMED, ...confirmed } },
  };
}

function marks(kind: FilmKind, vertebrae: VertebraOnFilm[], naming: Partial<FilmMarks['naming']> = {}): FilmMarks {
  return {
    film: kind,
    vertebrae,
    landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: 99, ...naming },
    namedFor: kind === 'side' ? 'anterior' : 'patient-left',
  };
}

/** A complete case: T1 to L5 on both films, every fact confirmed, L5 confirmed. */
function completeCase(): CaseFileV1 {
  return {
    format: 'spine-case',
    version: 1,
    savedWith: { app: 'test' },
    films: [film('front'), film('side')],
    marks: [marks('front', STANDARD.map((level, i) => vertebra(i, level, 'front'))), marks('side', STANDARD.map((level, i) => vertebra(100 + i, level, 'side')))],
    nextVertebraId: 200,
  };
}

const summary = (c: CaseFileV1) => checkCase(c).map((finding) => [finding.kind, finding.severity, finding.film, finding.vertebraId, finding.level]);

describe('the stand-in case checks', () => {
  it('find nothing in a complete, confirmed case', () => {
    expect(checkCase(completeCase())).toEqual([]);
  });

  it('report a missing level and an outline that still covers two', () => {
    const c = completeCase();
    const front = c.marks[0] as FilmMarks;
    const vertebrae = front.vertebrae.filter((each) => each.level !== 'T8').map((each) => (each.level === 'T9' ? { ...each, confidence: { ...each.confidence, reasons: ['odd-height' as const] } } : each));
    c.marks[0] = { ...front, vertebrae };
    expect(summary(c)).toEqual([
      ['vertebra-missing', 'blocking', 'front', 8, 'T9'],
      ['vertebra-missing', 'blocking', 'front', null, 'T8'],
      ['films-disagree', 'blocking', null, null, null],
    ]);
  });

  it('ask only for the levels from T1 to L5 a count expects, so a count with L6 does not ask for it', () => {
    const c = completeCase();
    const front = c.marks[0] as FilmMarks;
    const side = c.marks[1] as FilmMarks;
    // Extra-lumbar counts C7, T1 to T12 and L1 to L6: neither film has an L6, and none is asked for.
    c.marks[0] = { ...front, naming: { ...front.naming, template: 'extra-lumbar' } };
    c.marks[1] = { ...side, naming: { ...side.naming, template: 'extra-lumbar' } };
    expect(checkCase(c)).toEqual([]);
    // Extra-thoracic counts T13, between T1 and L5, so the side film is asked for it.
    c.marks[1] = { ...side, naming: { ...side.naming, template: 'extra-thoracic' } };
    expect(summary(c)).toEqual([['vertebra-missing', 'blocking', 'side', null, 'T13']]);
  });

  it('ask for L5 until it is confirmed, and warn of an off-by-one at the vertebra named L5 when the counts disagree', () => {
    const c = completeCase();
    const side = c.marks[1] as FilmMarks;
    c.marks[1] = { ...side, naming: { ...side.naming, l5ConfirmedOn: null, flags: ['ends-disagree'] } };
    // The side film's L5 is its seventeenth vertebra, id 100 + 16.
    expect(summary(c)).toEqual([
      ['likely-off-by-one', 'warning', 'side', 116, 'L5'],
      ['l5-unconfirmed', 'warning', 'side', null, null],
    ]);
    c.marks[1] = { ...side, naming: { ...side.naming, l5ConfirmedOn: 116, flags: ['ends-disagree'] } };
    expect(summary(c)).toEqual([]);
  });

  it('report levels out of order, flags from the naming, hidden pedicles and marks named for another facing', () => {
    const c = completeCase();
    const front = c.marks[0] as FilmMarks;
    const swapped = [...front.vertebrae];
    [swapped[3], swapped[4]] = [swapped[4] as VertebraOnFilm, swapped[3] as VertebraOnFilm];
    const first = swapped[0] as VertebraOnFilm;
    if (first.marks.kind !== 'front') throw new Error('not a front vertebra');
    swapped[0] = { ...first, marks: { kind: 'front', points: { ...first.marks.points, pedR: { seen: 'not-visible', source: 'hand' } } } };
    c.marks[0] = { ...front, vertebrae: swapped, namedFor: 'patient-right', naming: { ...front.naming, flags: ['names-unknown', 'non-standard-count'] } };
    expect(summary(c)).toEqual([
      ['levels-out-of-order', 'blocking', 'front', null, null],
      ['non-standard-count', 'warning', 'front', null, null],
      ['names-unknown', 'blocking', 'front', null, null],
      ['marks-named-for-other-facing', 'blocking', 'front', null, null],
      ['pedicle-not-visible', 'warning', 'front', 0, 'T1'],
      ['films-disagree', 'blocking', null, null, null],
    ]);
  });

  it('point a non-standard count at the first vertebra outside it, and report each hidden pedicle on its own', () => {
    const c = completeCase();
    const front = c.marks[0] as FilmMarks;
    const vertebrae = [...front.vertebrae, vertebra(17, 'L6', 'front')];
    const first = vertebrae[0] as VertebraOnFilm;
    if (first.marks.kind !== 'front') throw new Error('not a front vertebra');
    const hidden = { seen: 'not-visible', source: 'auto' } as const;
    vertebrae[0] = { ...first, marks: { kind: 'front', points: { ...first.marks.points, pedL: hidden, pedR: hidden } } };
    c.marks[0] = { ...front, vertebrae, naming: { ...front.naming, flags: ['non-standard-count'] } };
    expect(summary(c)).toEqual([
      ['non-standard-count', 'warning', 'front', 17, 'L6'],
      ['pedicle-not-visible', 'warning', 'front', 0, 'T1'],
      ['pedicle-not-visible', 'warning', 'front', 0, 'T1'],
      ['films-disagree', 'blocking', null, null, null],
    ]);
  });

  it('report marks named while the film’s facing is still unknown', () => {
    const c = completeCase();
    const side = c.films[1] as Film;
    c.films[1] = { ...side, geometry: { ...side.geometry, imageRight: null } };
    expect(summary(c)).toEqual([['marks-named-for-other-facing', 'blocking', 'side', null, null]]);
    // A film with no marks has nothing named, whatever its facing.
    c.marks = [c.marks[0] as FilmMarks];
    expect(summary(c)).toEqual([]);
  });

  it('find what the sample case says a check finds, at every stage of its story', () => {
    for (const stage of STORY_STAGES) expect(checkCase(caseAt(stage).case), stage).toEqual(findingsAt(stage));
  });

  it('report each fact no person has confirmed, by film', () => {
    const c = completeCase();
    c.films = [film('front', { leftRight: false, privacy: false }), film('side', { facing: false, upright: false, scale: false, polarity: false }), film('bend-left', { leftRight: false })];
    expect(summary(c)).toEqual([
      ['left-right-unconfirmed', 'blocking', 'front', null, null],
      ['privacy-unchecked', 'warning', 'front', null, null],
      ['upright-unconfirmed', 'warning', 'side', null, null],
      ['facing-unconfirmed', 'warning', 'side', null, null],
      ['scale-unconfirmed', 'warning', 'side', null, null],
      ['polarity-unconfirmed', 'warning', 'side', null, null],
      ['left-right-unconfirmed', 'warning', 'bend-left', null, null],
    ]);
  });
});
