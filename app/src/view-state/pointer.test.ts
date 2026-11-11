import { describe, expect, it } from 'vitest';
import { createPointer } from './pointer';

describe('pointer', () => {
  it('starts over no film', () => {
    expect(createPointer().getState().reading).toBeNull();
  });

  it('keeps the film, the point in film pixels and the point on screen', () => {
    const pointer = createPointer();
    pointer.getState().move({ film: 'bend-left', at: [120.5, 64], screen: [300, 210] });
    expect(pointer.getState().reading).toEqual({ film: 'bend-left', at: [120.5, 64], screen: [300, 210] });
  });

  it('forgets the reading when the pointer leaves the film', () => {
    const pointer = createPointer();
    pointer.getState().move({ film: 'front', at: [1, 2], screen: [3, 4] });
    pointer.getState().leave();
    expect(pointer.getState().reading).toBeNull();
  });
});
