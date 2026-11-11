// The reducer's rules, case by case, each from the header of
// case-edits.ts. The starting case has Front with vertebrae 1 to 3 (C7,
// T1, T2) and Side with 4 and 5 (C7, T1), so the next id is 6.

import { describe, expect, it } from 'vitest';
import type { FilmKind, FilmMarks, Level } from '@/shapes';
import type { CaseEdit, MarksCause } from './case-edits';
import { applyAction, redo, startAfresh, stateOf, undo, wandClicksWith, wandUnanswered, type CaseState } from './case-reducer';
import { marksOf } from './selectors';
import { sketchCase, sketchMarks, sketchVertebra, standardLevels, BOX, GAP } from './sketch-case';
import { HISTORY_LIMIT } from './undo-history';

const start = () =>
  stateOf(sketchCase(['front', 'side', 'bend-left'], [sketchMarks('front', 1, standardLevels('C7', 3)), sketchMarks('side', 4, standardLevels('C7', 2))]));

/** A film's marks as an engine answer gives them: a box per id, named by `levels`. */
function answerMarks(film: FilmKind, ids: readonly number[], levels: readonly Level[], shift = 0): FilmMarks {
  const marks = sketchMarks(film, 0, levels.slice(0, ids.length));
  return { ...marks, vertebrae: ids.map((id, index) => sketchVertebra(film, id, levels[index] ?? 'C7', 100 + shift, 100 + index * (BOX + GAP))) };
}

const answer = (film: FilmKind, ids: readonly number[], basedOn: number, cause: MarksCause = { by: 'auto-pass' }, shift = 0): CaseEdit => ({
  kind: 'replace-marks',
  marks: answerMarks(film, ids, standardLevels('C7', ids.length), shift),
  basedOn,
  cause,
});

function apply(state: CaseState, ...edits: CaseEdit[]): CaseState {
  return applyAction(state, edits).state;
}

const ids = (state: CaseState, film: FilmKind) => marksOf(state.case, film)?.vertebrae.map((vertebra) => vertebra.id) ?? null;

const flip = (film: FilmKind): CaseEdit => ({ kind: 'flip-brightness', film });
const move = (vertebra: number, x: number, gesture: number | null): CaseEdit => ({
  kind: 'move-mark',
  film: 'front',
  target: { on: 'vertebra', vertebra, mark: 'supL' },
  to: [x, 100],
  gesture,
});

describe('the version', () => {
  it('goes up once per kept edit of an action, and not for a dropped one', () => {
    const { state, report } = applyAction(start(), [flip('front'), answer('side', [4, 5], 0), flip('bend-right'), flip('side')]);
    expect(report.outcomes).toEqual([{ kept: true }, { kept: true }, { kept: false, why: 'not-in-case' }, { kept: true }]);
    expect(state.version).toBe(3);
    expect(state.changedAt).toEqual({ front: 1, side: 3, 'bend-left': 0, 'bend-right': 0 });
  });

  it('goes up by one for an undo and for a redo, and is never restored', () => {
    const applied = apply(start(), flip('front'), flip('side'));
    const undone = undo(applied).state;
    expect(undone.version).toBe(3);
    expect(redo(undone).state.version).toBe(4);
  });
});

describe('an action that keeps nothing', () => {
  it('changes nothing at all, adds no undo step and leaves redo alone', () => {
    const undone = undo(apply(start(), flip('front'))).state;
    const { state, report } = applyAction(undone, [answer('front', [1, 2, 3], 0), flip('bend-right')]);
    expect(state).toBe(undone);
    expect(report).toEqual({ outcomes: [{ kept: false, why: 'film-changed' }, { kept: false, why: 'not-in-case' }], record: null });
    expect(state.history.redo).toHaveLength(1);
  });
});

describe('staleness, judged per film', () => {
  it('lets an answer land after work on other films', () => {
    const state = apply(start(), flip('front'), flip('bend-left'));
    const { report } = applyAction(state, [answer('side', [4, 5], 0, { by: 'auto-pass' }, 5)]);
    expect(report.outcomes).toEqual([{ kept: true }]);
  });

  it('drops an answer whose own film changed after it was asked', () => {
    const state = apply(start(), flip('side'));
    const { state: after, report } = applyAction(state, [answer('side', [4, 5], 0)]);
    expect(report.outcomes).toEqual([{ kept: false, why: 'film-changed' }]);
    expect(after).toBe(state);
  });

  it('drops an answer holding an id given out since to another film, and keeps one above it', () => {
    // A split on Side lands first and gives out id 6.
    const split = apply(start(), answer('side', [4, 5, 6], 0, { by: 'boundary-wand', action: 'split', vertebra: 5 }));
    expect(split.case.nextVertebraId).toBe(7);
    const late = applyAction(split, [answer('front', [1, 2, 3, 6], 0, { by: 'boundary-wand', action: 'split', vertebra: 3 })]);
    expect(late.report.outcomes).toEqual([{ kept: false, why: 'id-given-out' }]);
    const above = applyAction(split, [answer('front', [1, 2, 3, 7], 0, { by: 'boundary-wand', action: 'split', vertebra: 3 })]);
    expect(above.report.outcomes).toEqual([{ kept: true }]);
  });

  it('lands every film of a pass, each film taking ids above the one before', () => {
    const { state, report } = applyAction(start(), [answer('front', [6, 7, 8], 0), answer('side', [9, 10], 0)]);
    expect(report.outcomes).toEqual([{ kept: true }, { kept: true }]);
    expect([ids(state, 'front'), ids(state, 'side')]).toEqual([[6, 7, 8], [9, 10]]);
    expect(state.history.undo).toHaveLength(1);
  });

  it('counts an undo as a change to each film it restores', () => {
    const passed = apply(start(), answer('front', [6, 7, 8], 0), answer('side', [9, 10], 0));
    const undone = undo(passed).state;
    const late = applyAction(undone, [answer('side', [9, 10, 11], passed.version)]);
    expect(late.report.outcomes).toEqual([{ kept: false, why: 'film-changed' }]);
    const other = applyAction(undone, [answer('bend-left', [11], passed.version)]);
    expect(other.report.outcomes).toEqual([{ kept: true }]);
  });
});

describe('one undo list for the case', () => {
  it('undoes the newest action whole, restoring every film it touched, and redoes it', () => {
    const before = start();
    const passed = apply(before, answer('front', [6, 7, 8], 0), answer('side', [9, 10], 0));
    const undone = undo(passed).state;
    expect(undone.case.films).toStrictEqual(before.case.films);
    expect(undone.case.marks).toStrictEqual(before.case.marks);
    const redone = redo(undone).state;
    expect(redone.case.marks).toStrictEqual(passed.case.marks);
  });

  it('acts on the newest action whichever film it touched', () => {
    const state = apply(apply(start(), flip('side')), flip('front'));
    const undone = undo(state).state;
    expect(undone.case.films[0]?.values.inverted).toBe(false);
    expect(undone.case.films[1]?.values.inverted).toBe(true);
  });

  it('clears redo when an action keeps an edit', () => {
    const undone = undo(apply(start(), flip('front'))).state;
    const state = apply(undone, flip('side'));
    expect(state.history.redo).toHaveLength(0);
    expect(redo(state).record).toBeNull();
  });

  it('keeps every id above the ones given out, even when an undo takes the vertebra away', () => {
    const split = apply(start(), answer('side', [4, 5, 6], 0, { by: 'boundary-wand', action: 'split', vertebra: 5 }));
    const undone = undo(split).state;
    expect(ids(undone, 'side')).toEqual([4, 5]);
    expect(undone.case.nextVertebraId).toBe(7);
  });

  it('drops a late answer holding an id that an undo took away from another film', () => {
    // The split gave 6 to Side; the undo takes it away, and no film holds
    // it now. A late Front answer asked before the split holds 6 too:
    // Front has not changed, but 6 is below 7 and Front does not hold it,
    // so it was given out since and the answer must go.
    const split = apply(start(), answer('side', [4, 5, 6], 0, { by: 'boundary-wand', action: 'split', vertebra: 5 }));
    const undone = undo(split).state;
    expect([ids(undone, 'front'), ids(undone, 'side'), undone.case.nextVertebraId]).toEqual([[1, 2, 3], [4, 5], 7]);
    const late = applyAction(undone, [answer('front', [1, 2, 3, 6], 0, { by: 'boundary-wand', action: 'split', vertebra: 3 })]);
    expect(late.report.outcomes).toEqual([{ kept: false, why: 'id-given-out' }]);
    expect(late.state).toBe(undone);
    expect(ids(redo(late.state).state, 'side')).toEqual([4, 5, 6]);
  });

  it(`keeps at most ${String(HISTORY_LIMIT)} actions to undo`, () => {
    let state = start();
    for (let index = 0; index <= HISTORY_LIMIT; index += 1) state = apply(state, flip('bend-left'));
    let undone = 0;
    for (;;) {
      const next = undo(state);
      if (next.record === null) break;
      state = next.state;
      undone += 1;
    }
    expect(undone).toBe(HISTORY_LIMIT);
  });
});

describe('a drag', () => {
  it('joins consecutive moves of one gesture into one undo step', () => {
    const before = start();
    const dragged = apply(apply(apply(before, move(1, 101, 7)), move(1, 102, 7)), move(1, 103, 7));
    expect(dragged.history.undo).toHaveLength(1);
    expect(dragged.version).toBe(3);
    expect(undo(dragged).state.case.marks).toStrictEqual(before.case.marks);
  });

  it('starts a new step for another gesture, for a move that stands alone, and after an undo', () => {
    const two = apply(apply(start(), move(1, 101, 7)), move(1, 102, 8));
    expect(two.history.undo).toHaveLength(2);
    const alone = apply(apply(start(), move(1, 101, null)), move(1, 102, null));
    expect(alone.history.undo).toHaveLength(2);
    const afterUndo = apply(undo(apply(apply(start(), flip('side')), move(1, 101, 7))).state, move(1, 102, 7));
    expect(afterUndo.history.undo).toHaveLength(2);
  });

  it('joins only consecutive moves: not after a redo of the drag, nor after an undo of the action that followed it', () => {
    // A gesture-7 drag, then a flip of Side. Redoing the drag, or undoing
    // the flip, puts the drag newest again, but a move of gesture 7 that
    // follows is not part of it.
    const dragged = apply(start(), move(1, 101, 7));
    const redone = redo(undo(dragged).state).state;
    expect(apply(redone, move(1, 102, 7)).history.undo).toHaveLength(2);
    const flippedBack = undo(apply(dragged, flip('side'))).state;
    expect(flippedBack.history.undo).toHaveLength(1);
    expect(apply(flippedBack, move(1, 102, 7)).history.undo).toHaveLength(2);
  });

  it('is joined across an action that kept nothing, which leaves no trace', () => {
    const state = apply(apply(apply(start(), move(1, 101, 7)), answer('front', [1, 2, 3], 0)), move(1, 102, 7));
    expect(state.history.undo).toHaveLength(1);
  });
});

describe('a fresh history', () => {
  it('leaves the case and the version as they are, with nothing to undo or redo', () => {
    const state = undo(apply(start(), flip('front'), flip('side'))).state;
    const fresh = startAfresh(state);
    expect(fresh.case).toBe(state.case);
    expect(fresh.version).toBe(state.version);
    expect(undo(fresh).record).toBeNull();
    expect(redo(fresh).record).toBeNull();
  });

  it("comes with an opened case, which takes its own next id and changes every film", () => {
    const worked = apply(start(), flip('front'));
    const opened = sketchCase(['side'], [sketchMarks('side', 1, standardLevels('C7', 1))]);
    const { state, report } = applyAction(worked, [{ kind: 'open-case', case: opened }]);
    expect(report.record?.kind).toBe('open-case');
    expect(state.case).toStrictEqual(opened);
    expect(state.history.undo).toHaveLength(0);
    expect(Object.values(state.changedAt)).toEqual([2, 2, 2, 2]);
    expect(applyAction(state, [answer('side', [1], 1)]).report.outcomes).toEqual([{ kept: false, why: 'film-changed' }]);
  });

  it('keeps the edits after an opened case in the same action as one step back to the opened case', () => {
    const opened = sketchCase(['side']);
    const state = apply(start(), { kind: 'open-case', case: opened }, flip('side'));
    expect(state.history.undo).toHaveLength(1);
    expect(undo(state).state.case).toStrictEqual(opened);
  });
});

describe('what each action was', () => {
  it('names a split on Front and Side by its cause, its films and the level it was aimed at', () => {
    const split: MarksCause = { by: 'boundary-wand', action: 'split', vertebra: 2 };
    const { state, report } = applyAction(start(), [answer('front', [1, 2, 6, 3], 0, split), answer('side', [4, 5, 7], 0, { by: 'boundary-wand', action: 'split', vertebra: 5 })]);
    expect(report.record).toEqual({ kind: 'replace-marks', cause: split, films: ['front', 'side'], aimedAt: { film: 'front', vertebra: 2, level: 'T1' } });
    expect(undo(state).record).toEqual(report.record);
  });

  it('takes the name the caller gives, and the level from before the action', () => {
    const { report } = applyAction(start(), [{ kind: 'confirm-l5', film: 'front', vertebra: 3 }], {});
    expect(report.record?.aimedAt).toEqual({ film: 'front', vertebra: 3, level: 'T2' });
    const looksRight = applyAction(start(), [move(2, 100, null)], { as: 'confirm-vertebra' });
    expect(looksRight.report.record).toEqual({ kind: 'confirm-vertebra', cause: null, films: ['front'], aimedAt: { film: 'front', vertebra: 2, level: 'T1' } });
  });

  it('keeps the first record of a drag', () => {
    const state = apply(apply(start(), move(1, 101, 7)), move(2, 102, 7));
    expect(state.history.undo[0]?.record.aimedAt?.vertebra).toBe(1);
  });
});

describe('region wand clicks', () => {
  const click = (x: number, polarity: 'add' | 'subtract' = 'add') => ({ at: [x, 120] as [number, number], polarity });
  const wandAnswer = (basedOn: number, shift: number) => answer('front', [1, 2, 3], basedOn, { by: 'region-wand', vertebra: 2 }, shift);

  function clicked(state: CaseState, x: number): CaseState {
    const asked = wandClicksWith(state, 'front', 2, click(x));
    if (asked === null) throw new Error('vertebra 2 is on Front');
    return apply(asked.state, wandAnswer(state.version, x - 100));
  }

  it('become the vertebra session when its answer lands, and grow with each click', () => {
    const once = clicked(start(), 101);
    expect(once.wand.clicks[2]).toEqual([click(101)]);
    const asked = wandClicksWith(once, 'front', 2, click(103));
    expect(asked?.clicks).toEqual([click(101), click(103)]);
  });

  it('are not kept when the answer is dropped', () => {
    const asked = wandClicksWith(start(), 'front', 2, click(101));
    if (asked === null) throw new Error('vertebra 2 is on Front');
    const state = apply(apply(asked.state, flip('front')), wandAnswer(0, 1));
    expect(state.wand.clicks[2]).toBeUndefined();
  });

  it('a second click before the first answer is in: the first answer makes the first clicks the session', () => {
    // Clicks at x = 101 and 102, both asked at version 0. The first answer
    // lands (version 1), so the session is [101]; the second answer is then
    // stale, and the next click is sent as [101, 103].
    const first = wandClicksWith(start(), 'front', 2, click(101));
    if (first === null) throw new Error('vertebra 2 is on Front');
    const second = wandClicksWith(first.state, 'front', 2, click(102));
    expect(second?.clicks).toEqual([click(102)]);
    if (second === null) throw new Error('vertebra 2 is on Front');
    const landed = apply(second.state, wandAnswer(0, 1));
    expect(landed.wand.clicks[2]).toEqual([click(101)]);
    const late = applyAction(landed, [wandAnswer(0, 2)]);
    expect(late.report.outcomes).toEqual([{ kept: false, why: 'film-changed' }]);
    expect(late.state).toBe(landed);
    expect(wandClicksWith(landed, 'front', 2, click(103))?.clicks).toEqual([click(101), click(103)]);
  });

  it('a click asked after the film changed is answered by its own answer, not by an older one still waiting', () => {
    // 101 asked at version 0; Front flips (version 1); 102 asked at version 1.
    // The answer to 101 is dropped, the answer to 102 lands: the session is [102].
    const first = wandClicksWith(start(), 'front', 2, click(101));
    if (first === null) throw new Error('vertebra 2 is on Front');
    const flipped = apply(first.state, flip('front'));
    const second = wandClicksWith(flipped, 'front', 2, click(102));
    if (second === null) throw new Error('vertebra 2 is on Front');
    expect(second.state.wand.sent[2]).toEqual([{ ask: 1, askedAt: 1, clicks: [click(102)] }]);
    const dropped = applyAction(second.state, [wandAnswer(0, 1)]);
    expect(dropped.report.outcomes).toEqual([{ kept: false, why: 'film-changed' }]);
    expect(apply(dropped.state, wandAnswer(1, 2)).wand.clicks[2]).toEqual([click(102)]);
  });

  it('a click the engine refused: the retry at the same version is answered by its own answer', () => {
    // 101 asked at version 0 and refused; the tool says so. 150 asked at
    // version 0 lands: the session is [150], and the next click is sent
    // as [150, 160], with nothing of the refused click.
    const refused = wandClicksWith(start(), 'front', 2, click(101));
    if (refused === null) throw new Error('vertebra 2 is on Front');
    const retry = wandClicksWith(wandUnanswered(refused.state, refused.ask), 'front', 2, click(150));
    if (retry === null) throw new Error('vertebra 2 is on Front');
    expect(retry.clicks).toEqual([click(150)]);
    expect(retry.ask).not.toBe(refused.ask);
    const landed = apply(retry.state, wandAnswer(0, 50));
    expect(landed.wand.clicks[2]).toEqual([click(150)]);
    expect(landed.wand.sent[2]).toBeUndefined();
    expect(wandClicksWith(landed, 'front', 2, click(160))?.clicks).toEqual([click(150), click(160)]);
  });

  it('a request that is not waiting changes nothing when said to have no answer', () => {
    const state = start();
    expect(wandUnanswered(state, 0)).toBe(state);
  });

  it('numbers requests so that no number is given twice, across an opened case', () => {
    const first = wandClicksWith(start(), 'front', 2, click(101));
    if (first === null) throw new Error('vertebra 2 is on Front');
    const reopened = apply(first.state, { kind: 'open-case', case: start().case });
    expect(reopened.wand.sent).toEqual({});
    expect(wandClicksWith(reopened, 'front', 2, click(101))?.ask).toBe(1);
  });

  it('are cleared when another tool changes that vertebra, and only that one', () => {
    const state = clicked(start(), 101);
    expect(apply(state, move(1, 90, null)).wand.clicks[2]).toEqual([click(101)]);
    expect(apply(state, move(2, 90, null)).wand.clicks[2]).toBeUndefined();
    expect(apply(state, { kind: 'pin-mark', film: 'front', target: { on: 'vertebra', vertebra: 2, mark: 'infR' }, pinned: true }).wand.clicks[2]).toBeUndefined();
  });

  it('are cleared by an undo that changes that vertebra, and kept by one that does not', () => {
    const twice = clicked(clicked(start(), 101), 104);
    expect(twice.wand.clicks[2]).toHaveLength(2);
    expect(undo(twice).state.wand.clicks[2]).toBeUndefined();
    const elsewhere = apply(clicked(start(), 101), flip('side'));
    expect(undo(elsewhere).state.wand.clicks[2]).toEqual([click(101)]);
    expect(redo(undo(elsewhere).state).state.wand.clicks[2]).toEqual([click(101)]);
  });

  it('are nothing to ask with for a vertebra the film does not hold', () => {
    expect(wandClicksWith(start(), 'side', 2, click(101))).toBeNull();
  });
});
