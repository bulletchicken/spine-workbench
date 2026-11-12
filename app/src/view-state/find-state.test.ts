import { describe, expect, it } from 'vitest';
import { createFindState } from './find-state';

describe('the find for this visit', () => {
  it('keeps progress increasing, between zero and one', () => {
    const s = createFindState();
    s.getState().searching(4);
    for (const share of [0.4, 0.2, NaN, -1]) s.getState().progress(share);
    expect(s.getState().share).toBe(0.4);
    s.getState().progress(2);
    expect(s.getState().share).toBe(1);
    s.getState().found();
    s.getState().progress(0.1);
    expect(s.getState().share).toBe(1);
  });
  it('fixes a review list, remembers looks, and clears them on another search', () => {
    const s = createFindState();
    const levels = ['T1', 'L5'] as const;
    s.getState().openReview(levels);
    s.getState().lookedAt('T1');
    s.getState().lookedAt('T1');
    expect(s.getState()).toMatchObject({ reviewing: levels, review: 0, looked: ['T1'], holding: true });
    s.getState().look(1);
    expect(s.getState()).toMatchObject({ review: 1, holding: false });
    s.getState().look(null);
    s.getState().found();
    expect(s.getState().looked).toEqual(['T1']);
    s.getState().searching(1);
    expect(s.getState()).toMatchObject({ phase: 'searching', searched: 1, looked: [], review: null });
    s.getState().failed('engine-crashed');
    expect(s.getState().failure).toBe('engine-crashed');
    s.getState().clear();
    expect(s.getState()).toMatchObject({ phase: 'idle', share: 0, failure: null, looked: [] });
  });
  it('keeps the looks through a rest only when the search had finished', () => {
    const s = createFindState();
    s.getState().searching(4);
    s.getState().found();
    s.getState().openReview(['T6']);
    s.getState().lookedAt('T6');
    s.getState().rest();
    expect(s.getState()).toMatchObject({ phase: 'idle', review: null, reviewing: [], holding: false, looked: ['T6'] });
    s.getState().searching(4);
    s.getState().lookedAt('T6');
    s.getState().rest();
    expect(s.getState()).toMatchObject({ phase: 'idle', looked: [] });
  });
});
