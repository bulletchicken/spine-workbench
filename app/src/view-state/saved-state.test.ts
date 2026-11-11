import { describe, expect, it } from 'vitest';
import { createSavedState, saveStatus } from './saved-state';

describe('save status', () => {
  it('is not saved yet before any save or open', () => {
    expect(saveStatus(null, 0)).toBe('not-saved-yet');
    expect(saveStatus(null, 7)).toBe('not-saved-yet');
  });

  it('is up to date at the version of the last save or open, whichever it was', () => {
    expect(saveStatus({ version: 7, how: 'saved' }, 7)).toBe('up-to-date');
    expect(saveStatus({ version: 7, how: 'opened' }, 7)).toBe('up-to-date');
  });

  it('reads changes not saved from the first edit after the mark', () => {
    expect(saveStatus({ version: 7, how: 'saved' }, 8)).toBe('changes-not-saved');
    expect(saveStatus({ version: 7, how: 'opened' }, 9)).toBe('changes-not-saved');
  });
});

describe('saved state', () => {
  it('starts untitled and never saved', () => {
    const state = createSavedState().getState();
    expect(state.name).toBeNull();
    expect(state.mark).toBeNull();
  });

  it('marks a save and an open with what they were', () => {
    const saved = createSavedState();
    saved.getState().markSaved(12);
    expect(saved.getState().mark).toEqual({ version: 12, how: 'saved' });
    saved.getState().markOpened(3);
    expect(saved.getState().mark).toEqual({ version: 3, how: 'opened' });
  });

  it('shows a name, and goes back to untitled', () => {
    const saved = createSavedState();
    saved.getState().setName('Made-up spine');
    expect(saved.getState().name).toBe('Made-up spine');
    saved.getState().setName(null);
    expect(saved.getState().name).toBeNull();
  });
});
