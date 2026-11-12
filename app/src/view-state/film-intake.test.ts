import { describe, expect, it } from 'vitest';
import { createFilmIntake } from './film-intake';

describe('the film intake state', () => {
  it('starts empty and records a reading once per place', () => {
    const state = createFilmIntake();
    expect(state.getState()).toMatchObject({ from: null, sample: null, nextSample: 0, reading: [], making: false, leaving: false, notice: null });
    state.getState().source('yours');
    state.getState().begin(['side', 'front']);
    state.getState().read('side');
    state.getState().read('side');
    expect(state.getState()).toMatchObject({ from: 'yours', reading: ['front'], readDone: 1, readTotal: 2 });
  });
  it('clears everything except the sample round', () => {
    const state = createFilmIntake();
    state.getState().source('sample-film', 2);
    state.getState().next(3);
    state.getState().begin(['front']);
    state.getState().make(true);
    state.getState().leave(true);
    state.getState().tell({ kind: 'left-out', count: 2 });
    state.getState().clear();
    expect(state.getState()).toMatchObject({ from: null, sample: null, nextSample: 3, reading: [], readDone: 0, readTotal: 0, making: false, leaving: false, notice: null });
  });
});
