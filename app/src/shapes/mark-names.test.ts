import { describe, expect, it } from 'vitest';
import indexSource from './index.ts?raw';
import { LEVELS, TEMPLATE_LEVELS, TEMPLATE_NAMES } from './generated/constants';
import type { Level } from './generated/Level';
import type { Template } from './generated/Template';
import { FRONT_MARKS, MARK_FILM, SIDE_MARKS, compareLevels } from './mark-names';

describe('mark names and levels', () => {
  it('lists the marks in their written order', () => {
    expect(FRONT_MARKS).toEqual(['supL', 'supR', 'infL', 'infR', 'pedL', 'pedR']);
    expect(SIDE_MARKS).toEqual(['supA', 'supP', 'infA', 'infP']);
    expect(Object.keys(MARK_FILM)).toEqual([...FRONT_MARKS, ...SIDE_MARKS]);
  });

  it('lists the levels top to bottom', () => {
    expect(new Set<Level>(LEVELS).size).toBe(LEVELS.length);
    expect(LEVELS[0]).toBe('C1');
    expect(LEVELS.at(-1)).toBe('L7');
    expect(LEVELS).toHaveLength(27);
    expect([...LEVELS].reverse().sort(compareLevels)).toEqual(LEVELS);
    expect(compareLevels('T12', 'L1')).toBeLessThan(0);
    expect(compareLevels('C6', 'C7')).toBeLessThan(0);
  });

  it('gives each count its levels inside its names, in order', () => {
    for (const template of Object.keys(TEMPLATE_LEVELS) as Template[]) {
      const names = TEMPLATE_NAMES[template];
      const levels = TEMPLATE_LEVELS[template];
      const start = names.indexOf('C7');
      expect(names.slice(start, start + levels.length)).toEqual(levels);
      expect([...names].sort(compareLevels)).toEqual(names);
    }
    expect(TEMPLATE_LEVELS.standard).toHaveLength(18);
  });
});

describe('the shapes index', () => {
  it('re-exports every generated file', () => {
    const generated = Object.keys(import.meta.glob('./generated/*.ts')).map((path) =>
      path.slice('./generated/'.length, -'.ts'.length),
    );
    expect(generated.length).toBeGreaterThan(100);
    const missing = generated.filter(
      (name) =>
        !indexSource.includes(`from './generated/${name}';`) ||
        indexSource.split(`'./generated/${name}'`).length !== 2,
    );
    expect(missing).toEqual([]);
  });
});
