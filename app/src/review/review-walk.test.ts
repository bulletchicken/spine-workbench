import { beforeEach, describe, expect, it } from 'vitest';
import { useReviewWalk } from './review-walk';

// Worked by hand: a walk starts with nothing skipped and not on. Starting it
// on four items records four. Skipping the same key twice keeps one. Leaving
// the walk keeps what was skipped, so a walk resumed later still leaves it out.
// Forgetting, for a different case, clears all of it.

beforeEach(() => {
  useReviewWalk.setState({ reviewing: false, countAtStart: 0, skipped: [] });
});

describe('review walk', () => {
  it('starts not reviewing, with nothing skipped', () => {
    const state = useReviewWalk.getState();
    expect(state.reviewing).toBe(false);
    expect(state.skipped).toEqual([]);
  });

  it('records the count when a walk starts', () => {
    useReviewWalk.getState().start(4);
    expect(useReviewWalk.getState()).toMatchObject({ reviewing: true, countAtStart: 4 });
  });

  it('keeps each skipped key once', () => {
    useReviewWalk.getState().skip('needs:4');
    useReviewWalk.getState().skip('needs:4');
    useReviewWalk.getState().skip('look:2');
    expect(useReviewWalk.getState().skipped).toEqual(['needs:4', 'look:2']);
  });

  it('keeps the skips when the walk is left and started again', () => {
    useReviewWalk.getState().start(4);
    useReviewWalk.getState().skip('needs:4');
    useReviewWalk.getState().leave();
    expect(useReviewWalk.getState()).toMatchObject({ reviewing: false, skipped: ['needs:4'] });
    useReviewWalk.getState().start(3);
    expect(useReviewWalk.getState()).toMatchObject({ reviewing: true, countAtStart: 3, skipped: ['needs:4'] });
  });

  it('forgets the walk and every skip when a different case comes in', () => {
    useReviewWalk.getState().start(4);
    useReviewWalk.getState().skip('needs:4');
    useReviewWalk.getState().forget();
    expect(useReviewWalk.getState()).toMatchObject({ reviewing: false, countAtStart: 0, skipped: [] });
  });
});
