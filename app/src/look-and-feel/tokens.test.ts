// The tokens exist twice: the app's copy, which CSS reads, and the copy
// beside the design language in docs/design, which people read. The two
// may word a comment differently and must never differ in a value.
//
// Everything is seen on one of two grounds, paper or a film, and the two
// need opposite inks. So a token that depends on its ground has a value for
// each (`--ink-1-paper`, `--ink-1-film`), and the plain name (`--ink-1`) is
// handed over by the ground a part stands on. These tests hold the two
// hand-overs to the same names, so no token is ever paper's on a film.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

/** A stylesheet with its comments and spacing taken out, so only what CSS reads is left. */
function meaning(path: string): string {
  return readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8')
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

/** The custom properties a block sets, by name. */
function declarationsIn(block: string): Record<string, string> {
  return Object.fromEntries([...block.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)].map(([, name = '', value = '']) => [name, value.trim()]));
}

describe('the design tokens', () => {
  const app = meaning('./tokens.css');
  const docs = meaning('../../../docs/design/tokens.css');

  it('are the same in the app and beside the design language', () => {
    expect(docs).toBe(app);
  });

  it('are really compared: both copies hold the paper, the inks, the well and the one breakpoint', () => {
    for (const expected of ['--room-0: #ffffff;', '--ink-1-paper: #111417;', '--well: #000000;', '--breathe: 2.6s;', '@media (max-width: 1199px)']) {
      expect(app).toContain(expected);
    }
  });

  it('are one theme, light', () => {
    expect(app).toContain('color-scheme: light;');
    expect(app).not.toContain('prefers-color-scheme');
  });

  it('lift nothing, blur nothing and light nothing: the old names are kept and say none', () => {
    for (const none of ['--lift-1: none;', '--lift-2: none;', '--lift-3: none;', '--glass-blur: none;', '--glow-lumen: none;']) {
      expect(app).toContain(none);
    }
  });

  it('give lumen no hue: ink on paper, white on a film', () => {
    expect(app).toContain('--lumen-paper: #111417;');
    expect(app).toContain('--on-lumen-paper: #ffffff;');
    expect(app).toContain('--lumen-film: #ffffff;');
    expect(app).toContain('--lumen-hi-film: #ffffff;');
  });
});

describe('the two grounds', () => {
  const css = meaning('./tokens.css');
  const root = declarationsIn(/^:root \{([^}]*)\}/.exec(css)?.[1] ?? '');
  const onPaper = declarationsIn(/:root, \[data-ground="paper"\] \{([^}]*)\}/.exec(css)?.[1] ?? '');
  const onFilm = declarationsIn(/\[data-ground="film"\] \{([^}]*)\}/.exec(css)?.[1] ?? '');

  it('are paper by default, and a paper label on a film can turn back', () => {
    expect(Object.keys(onPaper).length).toBeGreaterThan(20);
  });

  it('hand over the same names', () => {
    expect(Object.keys(onFilm).sort()).toEqual(Object.keys(onPaper).sort());
  });

  it('hand over each ink, line, lumen and confidence token from its own ground’s value', () => {
    for (const name of Object.keys(onPaper).filter((each) => each !== '--focus-ring')) {
      expect(onPaper[name], name).toBe(`var(${name}-paper)`);
      expect(onFilm[name], name).toBe(`var(${name}-film)`);
      expect(root[`${name}-paper`], `${name}-paper`).toBeDefined();
      expect(root[`${name}-film`], `${name}-film`).toBeDefined();
    }
  });

  it('cover the inks, the lines, lumen and all five confidence colours', () => {
    for (const name of ['--ink-1', '--ink-2', '--ink-3', '--ink-4', '--line-1', '--line-2', '--line-3', '--lumen', '--on-lumen', '--sure', '--look', '--needs', '--by-you', '--inferred']) {
      expect(onPaper[name], name).toBeDefined();
    }
  });

  it('never set a ground’s token on the root alone, where a film would inherit paper’s', () => {
    for (const name of Object.keys(onPaper)) expect(root[name], name).toBeUndefined();
  });

  it('keep the film’s confidence colours as they were measured on film', () => {
    expect(root['--sure-film']).toBe('#5cc8ff');
    expect(root['--look-film']).toBe('#ffd23f');
    expect(root['--needs-film']).toBe('#f0626e');
    expect(root['--by-you-film']).toBe('#f4f9ff');
    expect(root['--inferred-film']).toBe('#aab4c3');
  });
});
