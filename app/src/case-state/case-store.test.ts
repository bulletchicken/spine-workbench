import { describe, expect, it } from 'vitest';
import type { CaseEdit } from './case-edits';
import { createCaseStore } from './case-store';
import { emptyCase } from './empty-case';
import { sketchCase, sketchMarks, standardLevels } from './sketch-case';
import { nextRedo, nextUndo } from './undo-history';

const flip: CaseEdit = { kind: 'flip-brightness', film: 'front' };

function storeWithFront() {
  return createCaseStore(sketchCase(['front'], [sketchMarks('front', 1, standardLevels('C7', 2))]));
}

/** How many times the store told its readers something changed. */
function told(store: ReturnType<typeof createCaseStore>): { count: number } {
  const counter = { count: 0 };
  store.subscribe(() => {
    counter.count += 1;
  });
  return counter;
}

describe('the case store', () => {
  it('starts with an empty case at version 0, nothing to undo', () => {
    const state = createCaseStore().getState();
    expect(state.case).toStrictEqual(emptyCase());
    expect(state.version).toBe(0);
    expect(state.undo()).toBeNull();
  });

  it('applies an action, reports it, and tells its readers once', () => {
    const store = storeWithFront();
    const counter = told(store);
    const report = store.getState().apply([flip, { kind: 'flip-brightness', film: 'side' }]);
    expect(report.outcomes).toEqual([{ kept: true }, { kept: false, why: 'not-in-case' }]);
    expect(report.record?.kind).toBe('flip-brightness');
    expect(store.getState().version).toBe(1);
    expect(store.getState().case.films[0]?.values.inverted).toBe(true);
    expect(counter.count).toBe(1);
  });

  it('tells no one about an action that keeps nothing', () => {
    const store = storeWithFront();
    const counter = told(store);
    store.getState().apply([{ kind: 'remove-film', film: 'side' }]);
    expect(counter.count).toBe(0);
    expect(store.getState().version).toBe(0);
  });

  it('undoes and redoes, saying what each was', () => {
    const store = storeWithFront();
    store.getState().apply([flip]);
    expect(store.getState().undo()?.kind).toBe('flip-brightness');
    expect(store.getState().case.films[0]?.values.inverted).toBe(false);
    expect(store.getState().redo()?.kind).toBe('flip-brightness');
    expect(store.getState().case.films[0]?.values.inverted).toBe(true);
    expect(store.getState().version).toBe(3);
  });

  it('names what Undo and Redo would do next', () => {
    const store = storeWithFront();
    expect(nextUndo(store.getState().history)).toBeNull();
    store.getState().apply([{ kind: 'confirm-l5', film: 'front', vertebra: 2 }]);
    const record = { kind: 'confirm-l5', cause: null, films: ['front'], aimedAt: { film: 'front', vertebra: 2, level: 'T1' } };
    expect(nextUndo(store.getState().history)).toEqual(record);
    store.getState().undo();
    expect([nextUndo(store.getState().history), nextRedo(store.getState().history)]).toEqual([null, record]);
  });

  it('starts the history afresh without changing the case', () => {
    const store = storeWithFront();
    store.getState().apply([flip]);
    const before = store.getState().case;
    store.getState().startAfresh();
    expect(store.getState().case).toBe(before);
    expect(store.getState().undo()).toBeNull();
  });

  it('keeps the clicks to send for a region wand click, and nothing for a vertebra it does not hold', () => {
    const store = storeWithFront();
    const click = { at: [120, 160] as [number, number], polarity: 'add' as const };
    expect(store.getState().wandClicksWith('front', 2, click)).toEqual({ clicks: [click], ask: 0 });
    expect(store.getState().wand.sent[2]).toEqual([{ ask: 0, askedAt: 0, clicks: [click] }]);
    expect(store.getState().wandClicksWith('front', 9, click)).toBeNull();
  });

  it('lets a region wand request that got no answer stop waiting, telling no one when it was not waiting', () => {
    const store = storeWithFront();
    const click = { at: [120, 160] as [number, number], polarity: 'add' as const };
    const asked = store.getState().wandClicksWith('front', 2, click);
    const counter = told(store);
    store.getState().wandUnanswered(asked?.ask ?? -1);
    expect(store.getState().wand.sent).toEqual({});
    expect(counter.count).toBe(1);
    store.getState().wandUnanswered(asked?.ask ?? -1);
    expect(counter.count).toBe(1);
  });
});
