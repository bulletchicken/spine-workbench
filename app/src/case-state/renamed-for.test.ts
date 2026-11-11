// The facing swap against the answers the engine's own `renamed_for`
// gives (engine/shapes/tests/side_swap.rs reads the same file), and the
// C7 landmark against means worked out by hand.

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import type { FilmMarks, ImageRight, Mark, VertebraOnFilm } from '@/shapes';
import { c7Of } from './c7-of';
import { renamedFor } from './renamed-for';

interface KnownAnswer {
  what: string;
  facing: ImageRight;
  before: FilmMarks;
  after: FilmMarks;
}

const SIDE_SWAP = new URL('../../../proving-ground/known-answers/side-swap.json', import.meta.url);
const { cases } = JSON.parse(readFileSync(SIDE_SWAP, 'utf8')) as { cases: KnownAnswer[] };

describe('renamedFor', () => {
  it('has known answers to meet', () => {
    expect(cases.length).toBeGreaterThan(0);
  });

  for (const { what, facing, before, after } of cases) {
    it(`gives the known answer: ${what}`, () => {
      expect(renamedFor(before, facing)).toStrictEqual(after);
    });

    it(`gives the marks back when renamed again: ${what}`, () => {
      expect(renamedFor(after, before.namedFor)).toStrictEqual(before);
    });
  }

  it('returns the very same marks when they are already named for that facing', () => {
    const [first] = cases;
    if (first === undefined) throw new Error('side-swap.json has a case');
    expect(renamedFor(first.before, first.before.namedFor)).toBe(first.before);
  });
});

const corner = (x: number, y: number, inferred = false): Mark => ({ at: [x, y], source: 'hand', pinned: true, inferred });

function sideVertebra(level: VertebraOnFilm['level'], inferred = false): VertebraOnFilm {
  return {
    id: 7,
    level,
    marks: {
      kind: 'side',
      points: { supA: corner(10, 20), supP: corner(50, 22, inferred), infA: corner(12, 61), infP: corner(49, 60) },
    },
    confidence: { score: 1, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
    outline: null,
  };
}

describe('c7Of', () => {
  it('is null when no vertebra is C7', () => {
    expect(c7Of([sideVertebra('T1')])).toBeNull();
  });

  it('is the mean of the C7 corners, worked out by hand, never pinned and from the engine', () => {
    // x: (10 + 50 + 49 + 12) / 4 = 30.25; y: (20 + 22 + 60 + 61) / 4 = 40.75.
    expect(c7Of([sideVertebra('C7'), sideVertebra('T1')])).toStrictEqual({
      at: [30.25, 40.75],
      source: 'auto',
      pinned: false,
      inferred: false,
    });
  });

  it('is inferred when any C7 corner is', () => {
    expect(c7Of([sideVertebra('C7', true)])?.inferred).toBe(true);
  });
});
