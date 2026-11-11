import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import type { Conf } from '@/look-and-feel/data-attributes';
import { countStates } from './tally';

// Worked by hand: sure, sure, look, needs, set by you and not counted give
// two sure, one worth a look, one needs you, one set by you. The last level
// is left out, so the four numbers add to five, not six.

describe('countStates', () => {
  it('counts each state, and leaves out a level that is not counted', () => {
    const states: Conf[] = ['sure', 'sure', 'look', 'needs', 'byyou', 'sacrum'];
    expect(countStates(states)).toEqual({ sure: 2, look: 1, needs: 1, byyou: 1 });
  });

  it('gives zeros for no levels', () => {
    expect(countStates([])).toEqual({ sure: 0, look: 0, needs: 0, byyou: 0 });
    expect(countStates(['sacrum', 'sacrum'])).toEqual({ sure: 0, look: 0, needs: 0, byyou: 0 });
  });

  it('adds up to the number of counted levels, whatever the order', () => {
    const state = fc.constantFrom<Conf>('sure', 'look', 'needs', 'byyou', 'sacrum');
    fc.assert(
      fc.property(fc.array(state, { maxLength: 20 }), (states) => {
        const counts = countStates(states);
        const counted = states.filter((s) => s !== 'sacrum').length;
        expect(counts.sure + counts.look + counts.needs + counts.byyou).toBe(counted);
        expect(countStates([...states].reverse())).toEqual(counts);
      }),
    );
  });
});
