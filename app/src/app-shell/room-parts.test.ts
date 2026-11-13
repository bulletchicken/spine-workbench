// The room's three parts are found by path. Each is optional until its
// slice lands, and nothing else is ever mounted this way.

import { describe, expect, it } from 'vitest';
import { findRoomParts, ROOM_PART_PATHS, roomPartsFrom } from './room-parts';

const Part = () => null;

describe('the parts of the room', () => {
  it('are found at exactly three paths', () => {
    expect(ROOM_PART_PATHS).toEqual({
      Bench: '../bench/Bench.tsx',
      CaseSheet: '../case-sheet/CaseSheet.tsx',
      OpeningCase: '../reopen-case/OpeningCase.tsx',
    });
  });

  it('are each missing until their file exists', () => {
    expect(roomPartsFrom({})).toEqual({ Bench: null, CaseSheet: null, OpeningCase: null });
    expect(roomPartsFrom({ '../bench/Bench.tsx': Part })).toEqual({ Bench: Part, CaseSheet: null, OpeningCase: null });
  });

  it('take a file only from its own path: a misnamed file is not mounted', () => {
    expect(roomPartsFrom({ '../bench/bench.tsx': Part, '../case-sheet/Sheet.tsx': Part })).toEqual({ Bench: null, CaseSheet: null, OpeningCase: null });
  });

  it('are looked for in the app at those three paths and no others', () => {
    expect(Object.keys(findRoomParts()).sort()).toEqual(Object.keys(ROOM_PART_PATHS).sort());
  });
});
