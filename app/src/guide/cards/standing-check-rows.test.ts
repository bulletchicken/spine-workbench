// The standing checks as the card words them, at each moment of the
// sample case's story. The expected sentences are the design's (its
// states 3.2 to 4.7) and the review's own: 17 vertebrae from T1 to L5,
// one of them missing until the tall outline is split.

import { describe, expect, it } from 'vitest';
import { standingRows } from '@/review/standing-checks';
import { caseAt, findingsAt, type StoryStage } from '@/sample-case/stages';
import { standingCheckRows } from './standing-check-rows';

const NAMES = { front: 'Front', side: 'Side', 'bend-left': 'Bending left', 'bend-right': 'Bending right' } as const;
const rowsAt = (stage: StoryStage) => standingCheckRows(standingRows(caseAt(stage).case.marks, findingsAt(stage)), (film) => NAMES[film]);

describe('the standing checks, worded', () => {
  it('after the pass: a vertebra is missing, and the names may be off by one', () => {
    expect(rowsAt('pass-result')).toEqual([
      { id: 'found', state: 'bad', title: '16 of 17 vertebrae found', lines: ['One missing between T1 and L5.'] },
      { id: 'in-order', state: 'ok', title: 'In order, top to bottom' },
      { id: 'names', state: 'warn', title: 'Level names may be off by one', lines: ['Counting from the first rib and from the sacrum disagree'] },
    ]);
  });

  it('once L5 is confirmed: the names are set by a person', () => {
    const rows = rowsAt('l5-set');
    expect(rows[0]).toEqual({ id: 'found', state: 'bad', title: '16 of 17 vertebrae found', lines: ['One missing between T1 and L5.'] });
    expect(rows[2]?.state).toBe('you');
    expect(rows[2]?.title).toBe('Level names set');
  });

  it('after the split: all 17 are found', () => {
    expect(rowsAt('after-split')[0]).toEqual({ id: 'found', state: 'ok', title: '17 of 17 vertebrae found' });
  });

  it('all clear: the standing three pass, and come first', () => {
    const rows = rowsAt('all-clear');
    expect(rows.slice(0, 3).map((row) => row.id)).toEqual(['found', 'in-order', 'names']);
    expect(rows.slice(0, 3).map((row) => row.state)).toEqual(['ok', 'ok', 'you']);
    // Anything after them is an extra row, and warns.
    expect(rows.slice(3).every((row) => row.state === 'warn')).toBe(true);
  });
});

describe('each kind of row', () => {
  const word = (row: Parameters<typeof standingCheckRows>[0][number]) => standingCheckRows([row], (film) => NAMES[film])[0];

  it('waits for L5 before it counts the vertebrae', () => {
    expect(word({ row: 'found', state: 'wait', found: 17, of: 17, missing: 0 })).toEqual({
      id: 'found',
      state: 'wait',
      title: '17 outlines found',
      lines: ['Counted once L5 is known'],
    });
  });

  it('says how many are missing, in words that agree with the number', () => {
    expect(word({ row: 'found', state: 'bad', found: 15, of: 17, missing: 2 })?.lines).toEqual(['2 missing between T1 and L5.']);
  });

  it('says why the levels are not in order', () => {
    expect(word({ row: 'in-order', state: 'bad', reason: 'out-of-order' })?.title).toBe('A level is repeated or out of order');
    expect(word({ row: 'in-order', state: 'bad', reason: 'films-disagree' })?.title).toBe('The front and side films disagree');
  });

  it('keeps the names set by a person, and says when the count is not the usual one', () => {
    expect(word({ row: 'names', state: 'you', reason: null })).toEqual({ id: 'names', state: 'you', title: 'Level names set', lines: ['L5 confirmed by you'] });
    expect(word({ row: 'names', state: 'you', reason: 'non-standard-count' })).toEqual({
      id: 'names',
      state: 'you',
      title: 'Level names set',
      lines: ['This count is not the usual one'],
    });
    expect(word({ row: 'names', state: 'warn', reason: 'non-standard-count' })?.title).toBe('This count is not the usual one');
    expect(word({ row: 'names', state: 'warn', reason: 'names-unknown' })?.title).toBe('Level names are unknown');
    expect(word({ row: 'names', state: 'ok', reason: null })).toEqual({ id: 'names', state: 'ok', title: 'Level names set' });
  });

  it('names the bending film an extra row is about, and gives each such row its own id', () => {
    const rows = standingCheckRows(
      [
        { row: 'other-facing', state: 'warn' },
        { row: 'bending-count', state: 'warn', film: 'bend-left' },
        { row: 'bending-count', state: 'warn', film: 'bend-right' },
      ],
      (film) => NAMES[film],
    );
    expect(rows.map((row) => row.title)).toEqual([
      'Marks are named for another facing',
      'Bending left counts a different number of levels from Front',
      'Bending right counts a different number of levels from Front',
    ]);
    expect(new Set(rows.map((row) => row.id)).size).toBe(3);
  });
});
