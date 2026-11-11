import { describe, expect, it } from 'vitest';
import { createSelection } from './selection';

describe('selection', () => {
  it('starts with nothing selected and nothing hot', () => {
    const state = createSelection().getState();
    expect(state.selected).toBeNull();
    expect(state.hot).toBeNull();
  });

  it('keeps the selected vertebra and its film apart from the hovered one', () => {
    const selection = createSelection();
    selection.getState().select({ film: 'front', vertebra: 3 });
    selection.getState().hover({ film: 'side', vertebra: 5 });
    expect(selection.getState().selected).toEqual({ film: 'front', vertebra: 3 });
    expect(selection.getState().hot).toEqual({ film: 'side', vertebra: 5 });
  });

  it('selects a vertebra without touching the hovered one', () => {
    const selection = createSelection();
    selection.getState().hover({ film: 'side', vertebra: 5 });
    selection.getState().select({ film: 'front', vertebra: 3 });
    expect(selection.getState().hot).toEqual({ film: 'side', vertebra: 5 });
    selection.getState().select(null);
    expect(selection.getState().selected).toBeNull();
    expect(selection.getState().hot).toEqual({ film: 'side', vertebra: 5 });
  });

  it('clears the hovered vertebra without touching the selection', () => {
    const selection = createSelection();
    selection.getState().select({ film: 'front', vertebra: 3 });
    selection.getState().hover({ film: 'front', vertebra: 3 });
    selection.getState().hover(null);
    expect(selection.getState().hot).toBeNull();
    expect(selection.getState().selected).toEqual({ film: 'front', vertebra: 3 });
  });
});
