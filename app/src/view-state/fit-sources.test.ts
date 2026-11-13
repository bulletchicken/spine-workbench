import { describe, expect, it } from 'vitest';
import { createFitSources } from './fit-sources';

describe('finding provenance', () => {
  it('keeps each film against its own opening in an independent visit', () => {
    const a = createFitSources(); const b = createFitSources();
    a.getState().kept('front', 'trained', 2);
    a.getState().kept('side', 'hand-built', 7);
    expect(a.getState().films).toEqual({ front: { finder: 'trained', opening: 2 }, side: { finder: 'hand-built', opening: 7 } });
    expect(b.getState().films).toEqual({});
    a.getState().forget('front');
    expect(a.getState().films).toEqual({ side: { finder: 'hand-built', opening: 7 } });
    a.getState().kept('side', 'hand-built', 8);
    expect(a.getState().films.side?.opening).toBe(8);
    a.getState().clear();
    expect(a.getState().films).toEqual({});
  });
});
