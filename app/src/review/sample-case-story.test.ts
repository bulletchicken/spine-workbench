import { describe, expect, it } from 'vitest';
import type { CaseFileV1 } from '@/shapes';
import { caseAt, findingsAt, type StoryStage } from '@/sample-case/stages';
import { reviewQueue } from './review-queue';
import { standingRows } from './standing-checks';
import { tally } from './tally';

// The sample case tells the same story as review-queue.test.ts, on all four
// films and with its own check's findings. These are the values worked out
// by hand there, with the same ids.
//
// The bending films are found right: 18 vertebrae, C7 to L5, and both
// counts C7 to L5. Front's chain never matches theirs: at the pass and once
// L5 is set its count from the sacrum is T1 to L6 (the merged outline and the
// one on the sacrum), and after the split it has 19 vertebrae. So no item
// links to a bending film at any stage. Until the split Front has 18
// vertebrae too, so no bending row; from the split on, each bending film has
// one fewer and gets the row.

interface Expected {
  queue: string[];
  bendingRows: string[];
  found: [number, number];
  names: string;
  tally: { sure: number; look: number; needs: number; byyou: number };
}

const EXPECTED: Partial<Record<StoryStage, Expected>> = {
  'pass-result': {
    queue: ['which-is-l5', 'missing', 'needs:4', 'look:2'],
    bendingRows: [],
    found: [16, 17],
    names: 'warn',
    tally: { sure: 13, look: 2, needs: 2, byyou: 0 },
  },
  'l5-set': { queue: ['missing', 'needs:4', 'look:2'], bendingRows: [], found: [16, 17], names: 'you', tally: { sure: 13, look: 1, needs: 2, byyou: 1 } },
  'after-split': {
    queue: ['needs:4', 'look:2'],
    bendingRows: ['bend-left', 'bend-right'],
    found: [17, 17],
    names: 'you',
    tally: { sure: 12, look: 1, needs: 1, byyou: 3 },
  },
  't3-fixed': { queue: ['look:2'], bendingRows: ['bend-left', 'bend-right'], found: [17, 17], names: 'you', tally: { sure: 12, look: 1, needs: 0, byyou: 4 } },
  'all-clear': { queue: [], bendingRows: ['bend-left', 'bend-right'], found: [17, 17], names: 'you', tally: { sure: 12, look: 0, needs: 0, byyou: 5 } },
};

describe('the review rules over the sample case', () => {
  for (const [stage, expected] of Object.entries(EXPECTED) as [StoryStage, Expected][]) {
    it(`at ${stage}: ${expected.queue.join(', ') || 'nothing to review'}`, () => {
      const films = caseAt(stage).case.marks;
      const findings = findingsAt(stage);
      const rows = standingRows(films, findings);
      const queue = reviewQueue(films, findings);
      expect(queue.map((item) => item.key)).toEqual(expected.queue);
      for (const item of queue) {
        const linked = item.kind === 'which-is-l5' ? [item.a, item.b] : [item.ids];
        for (const ids of linked) expect(Object.keys(ids).every((film) => film === 'front' || film === 'side')).toBe(true);
      }
      expect(rows.find((row) => row.row === 'found')).toMatchObject({ found: expected.found[0], of: expected.found[1] });
      expect(rows.find((row) => row.row === 'names')).toMatchObject({ state: expected.names });
      expect(rows.flatMap((row) => (row.row === 'bending-count' ? [row.film] : []))).toEqual(expected.bendingRows);
      expect(tally(films)).toEqual(expected.tally);
    });
  }

  it('follows the L5 candidates onto Side, and onto no bending film', () => {
    const first = reviewQueue(caseAt('pass-result').case.marks, findingsAt('pass-result'))[0];
    expect(first).toMatchObject({ kind: 'which-is-l5', a: { front: 17, side: 35 }, b: { front: 18, side: 36 } });
    if (first?.kind !== 'which-is-l5') return;
    expect(Object.keys(first.a).sort()).toEqual(['front', 'side']);
    expect(Object.keys(first.b).sort()).toEqual(['front', 'side']);
  });

  it('gives the same queue after the case is saved and opened again', () => {
    const saved = caseAt('after-split').case;
    const reopened = JSON.parse(JSON.stringify(saved)) as CaseFileV1;
    expect(reviewQueue(reopened.marks, findingsAt('after-split'))).toEqual(reviewQueue(saved.marks, findingsAt('after-split')));
  });
});
