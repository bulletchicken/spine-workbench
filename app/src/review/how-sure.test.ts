import fc from 'fast-check';
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import type { Confidence, ConfidenceBand, ConfidenceReason, FilmMarks, FrontMarks, Level, Mark, MarkSource, PedicleMark, Template, VertebraOnFilm } from '@/shapes';
import type { Conf } from '@/look-and-feel/data-attributes';
import { CONF_WORDS, REASON_PHRASES } from '@/words/how-sure';
import { howSure, levelState } from './how-sure';

// Worked by hand. A vertebra is read in this order: a level outside its
// film's count is not counted; a vertebra a person set is set by you; any
// other is sure, worth a look or needs you, from its band alone.

function mark(source: MarkSource = 'auto', pinned = false): Mark {
  return { at: [100, 100], source, pinned, inferred: false };
}

function visiblePedicle(source: MarkSource = 'auto', pinned = false): PedicleMark {
  return { seen: 'visible', at: [80, 100], source, pinned, inferred: false, ringScore: null };
}

function confidence(band: ConfidenceBand, reasons: ConfidenceReason[] = []): Confidence {
  const score = band === 'high' ? 0.95 : band === 'medium' ? 0.7 : 0.3;
  return { score, band, reasons, calibrated: false, endplateSpreadPx: null, margin: null };
}

/** A front vertebra whose six marks were all placed by the engine, unless `points` says otherwise. */
function front(id: number, level: Level, band: ConfidenceBand, points: Partial<FrontMarks> = {}, reasons: ConfidenceReason[] = []): VertebraOnFilm {
  return {
    id,
    level,
    confidence: confidence(band, reasons),
    outline: null,
    marks: {
      kind: 'front',
      points: {
        supL: mark(),
        supR: mark(),
        infL: mark(),
        infR: mark(),
        pedL: visiblePedicle(),
        pedR: visiblePedicle(),
        ...points,
      },
    },
  };
}

function vertebraWithId(count: FilmMarks, id: number): VertebraOnFilm {
  const found = count.vertebrae.find((vertebra) => vertebra.id === id);
  if (!found) throw new Error(`no vertebra with id ${String(id)}`);
  return found;
}

function film(template: Template, vertebrae: VertebraOnFilm[], l5ConfirmedOn: number | null = null): FilmMarks {
  return {
    film: 'front',
    vertebrae,
    landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template, chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn },
    namedFor: 'patient-left',
  };
}

describe('howSure: a level outside the count is not counted', () => {
  it('C7 is above T1, so it is not counted', () => {
    const c7 = front(1, 'C7', 'high');
    expect(howSure(film('standard', [c7]), c7)).toBe('sacrum');
  });

  it('L6 is not counted under the standard count, and is under extra-lumbar', () => {
    const l6 = front(2, 'L6', 'high');
    expect(howSure(film('standard', [l6]), l6)).toBe('sacrum');
    expect(howSure(film('extra-lumbar', [l6]), l6)).toBe('sure');
  });

  it('T13 is not counted unless the count has one', () => {
    const t13 = front(3, 'T13', 'high');
    expect(howSure(film('standard', [t13]), t13)).toBe('sacrum');
    expect(howSure(film('extra-thoracic', [t13]), t13)).toBe('sure');
  });

  it('T12 and L5 are not counted where the count lacks them', () => {
    const t12 = front(4, 'T12', 'high');
    const l5 = front(5, 'L5', 'high');
    expect(howSure(film('missing-thoracic', [t12]), t12)).toBe('sacrum');
    expect(howSure(film('missing-lumbar', [l5]), l5)).toBe('sacrum');
  });
});

describe('howSure: the band says sure, worth a look or needs you', () => {
  it('maps high, medium and low to sure, worth a look and needs you', () => {
    const count = film('standard', []);
    expect(howSure(count, front(1, 'T3', 'high'))).toBe('sure');
    expect(howSure(count, front(2, 'T3', 'medium'))).toBe('look');
    expect(howSure(count, front(3, 'T3', 'low'))).toBe('needs');
  });
});

describe('howSure: a person set it', () => {
  const low = (points: Partial<FrontMarks>) => front(1, 'T3', 'low', points);

  it('a pinned corner is set by you', () => {
    const v = low({ supL: mark('auto', true) });
    expect(howSure(film('standard', [v]), v)).toBe('byyou');
  });

  it('a corner placed by hand, by a pin or by a region wand is set by you', () => {
    for (const source of ['hand', 'pin', 'region-wand'] as const) {
      const v = low({ infR: mark(source) });
      expect(howSure(film('standard', [v]), v)).toBe('byyou');
    }
  });

  it('a corner a boundary wand moved, or the engine placed, is not', () => {
    for (const source of ['boundary-wand', 'auto'] as const) {
      const v = low({ infR: mark(source) });
      expect(howSure(film('standard', [v]), v)).toBe('needs');
    }
  });

  it('a pedicle placed by hand is set by you, and so is one marked not visible by hand', () => {
    const placed = low({ pedR: visiblePedicle('hand') });
    expect(howSure(film('standard', [placed]), placed)).toBe('byyou');
    const hidden = low({ pedL: { seen: 'not-visible', source: 'hand' } });
    expect(howSure(film('standard', [hidden]), hidden)).toBe('byyou');
  });

  it('a vertebra the user edited is set by you', () => {
    const v = front(1, 'T3', 'low', {}, ['edited-by-user']);
    expect(howSure(film('standard', [v]), v)).toBe('byyou');
  });

  it('the vertebra confirmed as L5 is set by you, and only that one', () => {
    const l5 = front(7, 'L5', 'low');
    const other = front(8, 'L4', 'low');
    const count = film('standard', [l5, other], 7);
    expect(howSure(count, l5)).toBe('byyou');
    expect(howSure(count, other)).toBe('needs');
  });

  it('a bending film: a pinned line end is set by you, placed ends are not', () => {
    const bend = (pinned: boolean): VertebraOnFilm => ({
      id: 1,
      level: 'T3',
      confidence: confidence('low'),
      outline: null,
      marks: {
        kind: 'bending',
        lines: {
          sup: [mark('auto', pinned), mark()],
          inf: [mark(), mark()],
        },
      },
    });
    const pinned = bend(true);
    const placed = bend(false);
    expect(howSure({ ...film('standard', [pinned]), film: 'bend-left' }, pinned)).toBe('byyou');
    expect(howSure({ ...film('standard', [placed]), film: 'bend-left' }, placed)).toBe('needs');
  });
});

describe('howSure: a tool aimed at one vertebra and a neighbour it only moved', () => {
  it('the aimed one stays set by you, and the neighbour is read from its band, after a save and reopen', () => {
    const aimed = front(1, 'T3', 'low', { supL: mark('region-wand') });
    const moved = front(2, 'T4', 'high', { supL: mark('boundary-wand'), supR: mark('boundary-wand') });
    const saved = JSON.parse(JSON.stringify(film('standard', [aimed, moved]))) as FilmMarks;
    expect(howSure(saved, vertebraWithId(saved, 1))).toBe('byyou');
    expect(howSure(saved, vertebraWithId(saved, 2))).toBe('sure');
  });
});

describe('levelState: the most urgent state across the films that count the level', () => {
  const cases: [Conf[], Conf][] = [
    [['needs', 'sure'], 'needs'],
    [['look', 'byyou'], 'look'],
    [['byyou', 'sure'], 'byyou'],
    [['sure', 'sure'], 'sure'],
    [['sacrum', 'look'], 'look'],
    [['sacrum', 'sacrum'], 'sacrum'],
    [['needs', 'look', 'byyou'], 'needs'],
  ];

  for (const [states, expected] of cases) {
    it(`${states.join(' and ')} gives ${expected}`, () => {
      expect(levelState(states)).toBe(expected);
    });
  }

  it('does not depend on the order of the films', () => {
    const state = fc.constantFrom<Conf>('sure', 'look', 'needs', 'byyou', 'sacrum');
    fc.assert(
      fc.property(fc.array(state, { minLength: 1, maxLength: 6 }), (states) => {
        expect(levelState(states)).toBe(levelState([...states].reverse()));
      }),
    );
  });

  it('is needs whenever a counted film says needs', () => {
    const state = fc.constantFrom<Conf>('sure', 'look', 'byyou', 'sacrum');
    fc.assert(
      fc.property(fc.array(state, { maxLength: 5 }), (others) => {
        expect(levelState([...others, 'needs'])).toBe('needs');
      }),
    );
  });
});

describe('the words for how sure', () => {
  it('gives each state the design\'s word', () => {
    expect(CONF_WORDS).toEqual({ sure: 'Sure', look: 'Worth a look', needs: 'Needs you', byyou: 'Set by you', sacrum: 'Not counted' });
  });

  it('has a phrase for every reason the case file can hold', () => {
    const schema = JSON.parse(readFileSync(new URL('../../../docs/contracts/case-file-v1.schema.json', import.meta.url), 'utf8')) as {
      $defs: { ConfidenceReason: { oneOf: { const: string }[] } };
    };
    const reasons = schema.$defs.ConfidenceReason.oneOf.map((entry) => entry.const).sort();
    expect(reasons).toHaveLength(13);
    expect(Object.keys(REASON_PHRASES).sort()).toEqual(reasons);
  });
});
