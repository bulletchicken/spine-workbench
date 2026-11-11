import { describe, expect, it } from 'vitest';
import type { CheckFinding, CheckKind, FilmKind, FilmMarks, Level, Template, VertebraOnFilm } from '@/shapes';
import { CHECK_SENTENCES } from '@/words/checks';
import { standingRows, whereFindingShows } from './standing-checks';

// A front film with one level empty: sixteen vertebrae, T1 to T11 then L1
// to L5, so T12 has none (ids 1 to 16 in that order). The design's own story
// is worked through in review-queue.test.ts.
//
// Counted from the first rib, the names are the ones on each vertebra: L5 is
// id 16. Counted up from the sacrum, the bottom vertebra is L6 and id 15 is
// L5. The two ends disagree about L5, so the question is open.
//
// Found: 16 of 17 (T1 to L5 is 17 levels; T12 has no vertebra).

const FROM_TOP: Level[] = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'L1', 'L2', 'L3', 'L4', 'L5'];
const FROM_BOTTOM: Level[] = ['T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'];

function vertebra(id: number, level: Level): VertebraOnFilm {
  const mark = { at: [0, 0] as [number, number], source: 'auto' as const, pinned: false, inferred: false };
  return {
    id,
    level,
    confidence: { score: 0.95, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
    outline: null,
    marks: { kind: 'bending', lines: { sup: [mark, mark], inf: [mark, mark] } },
  };
}

function film(kind: FilmKind, template: Template, levels: Level[], fromTop: Level[] | null, fromBottom: Level[] | null, l5ConfirmedOn: number | null): FilmMarks {
  return {
    film: kind,
    vertebrae: levels.map((level, index) => vertebra(index + 1, level)),
    landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template, chosenBy: 'engine', alternatives: [], fromTop, fromBottom, flags: [], l5ConfirmedOn },
    namedFor: 'patient-left',
  };
}

const story = (l5ConfirmedOn: number | null = null): FilmMarks => film('front', 'standard', FROM_TOP, FROM_TOP, FROM_BOTTOM, l5ConfirmedOn);

function finding(kind: CheckKind, film: FilmKind | null = 'front', level: Level | null = null): CheckFinding {
  return { kind, severity: 'warning', film, vertebraId: null, level };
}

const found = (rows: ReturnType<typeof standingRows>) => rows.find((row) => row.row === 'found');
const inOrder = (rows: ReturnType<typeof standingRows>) => rows.find((row) => row.row === 'in-order');
const names = (rows: ReturnType<typeof standingRows>) => rows.find((row) => row.row === 'names');

describe('standingRows: the three standing checks on the story', () => {
  it('the story before L5 is set: found is bad with 16 of 17, and names is warn off by one', () => {
    const rows = standingRows([story()], [finding('vertebra-missing', 'front', 'T12'), finding('likely-off-by-one')]);
    expect(found(rows)).toEqual({ row: 'found', state: 'bad', found: 16, of: 17, missing: 1 });
    expect(inOrder(rows)).toEqual({ row: 'in-order', state: 'ok', reason: null });
    expect(names(rows)).toEqual({ row: 'names', state: 'warn', reason: 'off-by-one' });
  });

  it('after L5 is set on id 16: names is you, and found still reads 16 of 17', () => {
    const rows = standingRows([story(16)], [finding('vertebra-missing', 'front', 'T12'), finding('likely-off-by-one')]);
    expect(found(rows)).toEqual({ row: 'found', state: 'bad', found: 16, of: 17, missing: 1 });
    expect(names(rows)).toEqual({ row: 'names', state: 'you', reason: null });
  });

  it('with L5 open and nothing missing, found waits, and it does not say bad', () => {
    const COMPLETE_TOP: Level[] = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];
    const COMPLETE_BOTTOM: Level[] = ['T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'];
    const complete = film('front', 'standard', COMPLETE_TOP, COMPLETE_TOP, COMPLETE_BOTTOM, null);
    const rows = standingRows([complete], []);
    expect(found(rows)).toEqual({ row: 'found', state: 'wait', found: 17, of: 17, missing: 0 });
  });

  it('names warns when the two counts disagree, even with no finding saying so', () => {
    expect(names(standingRows([story()], []))).toEqual({ row: 'names', state: 'warn', reason: 'off-by-one' });
  });

  it('once L5 is confirmed, an unusual count stays as the names row\'s second line', () => {
    expect(names(standingRows([story(16)], [finding('non-standard-count', 'front', 'L6')]))).toEqual({
      row: 'names',
      state: 'you',
      reason: 'non-standard-count',
    });
  });

  it('counts a level once when it is both empty and named by a finding', () => {
    const missingT12 = finding('vertebra-missing', 'front', 'T12');
    expect(found(standingRows([story(16)], [missingT12, missingT12]))).toMatchObject({ found: 16, missing: 1 });
  });

  // Front has all 17 levels (ids 1 to 17, T12 is id 12). Side has the same
  // with T12 missing. A finding on Side alone still makes one level missing:
  // 16 of 17, so the row never reads "17 of 17" while it is bad.
  it('counts a vertebra missing on Side alone, whether the finding names the level or not', () => {
    const full: Level[] = [...FROM_TOP.slice(0, 11), 'T12', ...FROM_TOP.slice(11)];
    const front = film('front', 'standard', full, null, null, 17);
    const side = film('side', 'standard', FROM_TOP, null, null, 16);
    const expected = { row: 'found', state: 'bad', found: 16, of: 17, missing: 1 };
    expect(found(standingRows([front, side], [finding('vertebra-missing', 'side', 'T12')]))).toEqual(expected);
    expect(found(standingRows([front, side], [finding('vertebra-missing', 'side', null)]))).toEqual(expected);
    expect(found(standingRows([front, front], [finding('vertebra-missing', 'side', 'T12')]))).toEqual(expected);
  });

  // An extra-lumbar count runs T1 to L6, 18 levels. The film has T1 to L5
  // only. L6 is past L5, so it is not missing as the engine means it: 17 of
  // 18 found, none missing, and nothing open, so the row is ok.
  it('counts only T1 to L5 as missing, the engine\'s own definition', () => {
    const full: Level[] = [...FROM_TOP.slice(0, 11), 'T12', ...FROM_TOP.slice(11)];
    const front = film('front', 'extra-lumbar', full, null, null, null);
    expect(found(standingRows([front], []))).toEqual({ row: 'found', state: 'ok', found: 17, of: 18, missing: 0 });
  });

  it('names reads its reasons in order: off by one first, then unknown, then non-standard', () => {
    const plain = film('front', 'standard', FROM_TOP, FROM_TOP, FROM_TOP, null);
    expect(names(standingRows([plain], [finding('names-unknown'), finding('non-standard-count')]))).toEqual({
      row: 'names',
      state: 'warn',
      reason: 'names-unknown',
    });
    expect(names(standingRows([plain], [finding('non-standard-count')]))).toEqual({
      row: 'names',
      state: 'warn',
      reason: 'non-standard-count',
    });
    expect(names(standingRows([plain], []))).toEqual({ row: 'names', state: 'ok', reason: null });
  });

  it('out-of-order and films-disagree both stop in order, and say which', () => {
    expect(inOrder(standingRows([story()], [finding('levels-out-of-order')]))).toEqual({
      row: 'in-order',
      state: 'bad',
      reason: 'out-of-order',
    });
    expect(inOrder(standingRows([story()], [finding('films-disagree', null)]))).toEqual({
      row: 'in-order',
      state: 'bad',
      reason: 'films-disagree',
    });
  });

  it('marks named for another facing add a warn row under the three', () => {
    const rows = standingRows([story(16)], [finding('marks-named-for-other-facing', 'side')]);
    expect(rows.at(-1)).toEqual({ row: 'other-facing', state: 'warn' });
    expect(rows).toHaveLength(4);
  });

  it('a bending film that counts a different number of levels adds one row, and one that matches adds none', () => {
    const bendLeft = film('bend-left', 'extra-thoracic', [...FROM_TOP.slice(0, 11), 'T12', 'T13', ...FROM_TOP.slice(11)], null, null, null);
    const bendRight = film('bend-right', 'standard', FROM_TOP, null, null, null);
    const rows = standingRows([story(16), bendLeft, bendRight], []);
    expect(rows.filter((row) => row.row === 'bending-count')).toEqual([{ row: 'bending-count', state: 'warn', film: 'bend-left' }]);
  });

  // Front after the split, as in the sample case: the C7 body, T1 to L5 and
  // the outline on the sacrum, 19 vertebrae named C7 to L6, L5 set on id 18.
  // A bending film found with the same merge Front had, and its L5 set too,
  // is left as found: 18 vertebrae named T1 to L6. It has one vertebra fewer,
  // so it gets the row.
  it('a bending film left as found after Front is split adds the row', () => {
    const C7_TO_L6: Level[] = ['C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'];
    const T1_TO_L6 = C7_TO_L6.slice(1);
    const front = film('front', 'standard', C7_TO_L6, C7_TO_L6, C7_TO_L6, 18);
    const bendRight = film('bend-right', 'standard', T1_TO_L6, C7_TO_L6.slice(0, 18), T1_TO_L6, 17);
    const rows = standingRows([front, bendRight], []);
    expect(rows.filter((row) => row.row === 'bending-count')).toEqual([{ row: 'bending-count', state: 'warn', film: 'bend-right' }]);
  });

  // The sample case at the pass: Front holds 18 vertebrae (the C7 body, the
  // merged T11 and T12, an outline on the sacrum), and a bending film found
  // right holds 18 (C7 to L5). They are not linked, since their counts from
  // the sacrum differ, but the bending film does not count a different number
  // of levels, so it gets no row.
  it('a bending film with as many vertebrae as Front gets no row', () => {
    const C7_TO_L5: Level[] = ['C7', ...FROM_TOP.slice(0, 11), 'T12', ...FROM_TOP.slice(11)];
    const T1_TO_L6: Level[] = [...C7_TO_L5.slice(1), 'L6'];
    const front = film('front', 'standard', C7_TO_L5, C7_TO_L5, T1_TO_L6, null);
    const bendLeft = film('bend-left', 'standard', C7_TO_L5, C7_TO_L5, C7_TO_L5, null);
    expect(standingRows([front, bendLeft], []).filter((row) => row.row === 'bending-count')).toEqual([]);
  });

  it('names still warns after Front\'s L5 is set when another film is off by one or unnamed', () => {
    const side = film('side', 'standard', FROM_TOP, FROM_TOP, FROM_BOTTOM, null);
    expect(names(standingRows([story(16), side], [finding('likely-off-by-one', 'side')]))).toEqual({
      row: 'names',
      state: 'warn',
      reason: 'off-by-one',
    });
    const bendLeft = film('bend-left', 'standard', FROM_TOP, null, null, null);
    expect(names(standingRows([story(16), bendLeft], [finding('names-unknown', 'bend-left')]))).toEqual({
      row: 'names',
      state: 'warn',
      reason: 'names-unknown',
    });
  });

  it('names lets an off-by-one finding go once that film\'s L5 is set', () => {
    const side = film('side', 'standard', FROM_TOP, FROM_TOP, FROM_BOTTOM, 15);
    expect(names(standingRows([story(16), side], [finding('likely-off-by-one', 'front'), finding('likely-off-by-one', 'side')]))).toEqual({
      row: 'names',
      state: 'you',
      reason: null,
    });
  });
});

describe('whereFindingShows: every check kind has a place and a sentence', () => {
  it('shows each check where the design says it goes', () => {
    expect(whereFindingShows(finding('vertebra-missing'))).toEqual({ place: 'found-row', stops: 'yes' });
    expect(whereFindingShows(finding('films-disagree', null))).toEqual({ place: 'in-order-row', stops: 'yes' });
    expect(whereFindingShows(finding('likely-off-by-one'))).toEqual({ place: 'names-row', stops: 'until-l5' });
    expect(whereFindingShows(finding('l5-unconfirmed'))).toEqual({ place: 'not-shown', stops: 'no' });
    expect(whereFindingShows(finding('pedicle-not-visible'))).toEqual({ place: 'on-film', stops: 'no' });
  });

  it('puts a left and right question on step two for Front, and on the bay for a bending film', () => {
    expect(whereFindingShows(finding('left-right-unconfirmed', 'front'))).toEqual({ place: 'step-two-question', stops: 'no' });
    expect(whereFindingShows(finding('left-right-unconfirmed', 'bend-left'))).toEqual({ place: 'bay-chip', stops: 'no' });
  });

  it('gives every check kind a sentence, and a place', () => {
    for (const kind of Object.keys(CHECK_SENTENCES) as CheckKind[]) {
      expect(CHECK_SENTENCES[kind].length).toBeGreaterThan(0);
      expect(whereFindingShows(finding(kind)).place).toBeTruthy();
    }
  });
});
