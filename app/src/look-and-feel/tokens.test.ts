// tokens.css and components.css are the design system's own files, kept in
// the app as they are written; DESIGN.md at the root of the repository says
// what each value and part is for. What the app needs beyond them lives
// apart: earlier-tokens.css serves parts no longer on the page. The screens
// never redefine the system’s values.

import { createHash } from 'node:crypto';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const read = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8');

/** A stylesheet with its comments and spacing taken out, so only what CSS reads is left. */
const meaning = (css: string) =>
  css
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\s+/g, ' ')
    .trim();

/** The custom properties a block sets, by name. */
function declarationsIn(block: string): Record<string, string> {
  return Object.fromEntries([...block.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)].map(([, name = '', value = '']) => [name, value.trim()]));
}

/** Every custom property a stylesheet sets, anywhere in it. */
function namesSetIn(css: string): Set<string> {
  return new Set([...css.matchAll(/(--[a-z0-9-]+)\s*:/g)].map(([, name = '']) => name));
}

/** Each innermost rule block of a stylesheet: its selector and what it sets. Media queries are looked inside. */
function blocksOf(css: string): { selector: string; sets: Record<string, string> }[] {
  return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map(([, selector = '', body = '']) => ({ selector: selector.trim(), sets: declarationsIn(body) }));
}

const TOKENS_RAW = read('./tokens.css');
const COMPONENTS_RAW = read('./components.css');
const TOKENS = meaning(TOKENS_RAW);
const COMPONENTS = meaning(COMPONENTS_RAW);
const EARLIER = meaning(read('./earlier-tokens.css'));
const ROOT = declarationsIn(/^:root \{([^}]*)\}/.exec(TOKENS)?.[1] ?? '');

/** The search DESIGN.md section 11 runs over components.css: a colour, a length or a time written as a number. */
const WRITTEN_AS_A_NUMBER = /#[0-9a-fA-F]{3,8}\b|rgba?\(|[0-9.]+(px|ms|em|rem|vw|vh)\b/;
const LET_BY = /\* 1px|100v[wh]/;

describe('the design system’s two stylesheets', () => {
  it('are its own files, holding the page, the ink, the film, the two signals, the frame and the casing', () => {
    expect(TOKENS_RAW.startsWith('/* Spine Workbench: tokens.')).toBe(true);
    expect(COMPONENTS_RAW.startsWith('/* Spine Workbench: components.')).toBe(true);
    for (const expected of ['--page: #fff;', '--ink: #0d0d0d;', '--film: #000;', '--accent: #0a7cff;', '--look: #f5a524;', '--frame-max-w: 1280px;', '--mark-casing-look: rgba(0, 0, 0, 0.65);']) {
      expect(TOKENS).toContain(expected);
    }
  });

  it('pass the system’s own search: components.css writes no colour, length or time as a number', () => {
    const found = COMPONENTS_RAW.split('\n').filter((line) => WRITTEN_AS_A_NUMBER.test(line) && !LET_BY.test(line));
    expect(found).toEqual([]);
  });

  it('are one theme, light', () => {
    expect(read('../../index.html')).toContain('<meta name="color-scheme" content="light" />');
    const here = fileURLToPath(new URL('.', import.meta.url));
    for (const name of readdirSync(here).filter((each) => each.endsWith('.css'))) {
      expect(readFileSync(join(here, name), 'utf8'), name).not.toContain('prefers-color-scheme');
    }
  });

  it('make three layers: a page, objects on a shadow, and glass that blurs what is behind it', () => {
    expect(ROOT['--glass-thin']).toBe('rgba(255, 255, 255, 0.66)');
    expect(ROOT['--glass-blur']).toBe('blur(24px) saturate(180%)');
    expect(ROOT['--shadow-film']).toBeDefined();
    expect(ROOT['--shadow-film']).not.toBe('none');
  });

  it('set everything in one family, and numbers in its mono face', () => {
    expect(ROOT['--font-sans']).toBe('Geist, ui-sans-serif, system-ui, sans-serif');
    expect(ROOT['--font-mono']).toBe('"Geist Mono", ui-monospace, monospace');
    expect(TOKENS.replaceAll('sans-serif', '')).not.toMatch(/serif/);
  });

  it('give every token one value: no ground hands one over', () => {
    for (const css of [TOKENS, COMPONENTS]) {
      expect(css).not.toContain('[data-ground');
      expect([...namesSetIn(css)].filter((name) => name.endsWith('-paper'))).toEqual([]);
    }
  });
});

describe('the refreshed system', () => {
  it('keeps all three files byte for byte', () => {
    for (const [path, digest] of [
      ['../../../DESIGN.md', '6895cf7173bba23ac4b4f038351c671b00b99fed46fbba52e4d6bf84a8eb9dd4'],
      ['./tokens.css', '199dfa63180da1630b89d2401f8901fb8d1d57b029cbde639a39e5aa8f191e27'],
      ['./components.css', 'f9f7efc55ddd2a593123aeb992ee762bde821ace369fd2f409c4b7b8097c0688'],
    ] as const) {
      expect(createHash('sha256').update(read(path)).digest('hex'), path).toBe(digest);
    }
  });

  it('owns the popover width and film height, and changes the frame, sentence and bar at its named widths', () => {
    expect(ROOT['--note-w']).toBe('312px');
    expect(ROOT['--film-h']).toBe('1300');
    expect(ROOT['--type-say']).toBe('var(--type-heading)');
    expect(ROOT['--track-say']).toBe('var(--track-heading)');
    expect(ROOT['--bar-cols']).toBe('1fr auto 1fr');
    expect(ROOT['--brand-name-display']).toBe('inline');
    expect(ROOT['--break-narrow-w']).toBe('900px');
    expect(ROOT['--break-small-w']).toBe('600px');
    const atWidth = (width: number) => declarationsIn(new RegExp(String.raw`@media \(max-width: ${String(width)}px\) \{ :root \{([^}]*)\}`).exec(TOKENS)?.[1] ?? '');
    expect(atWidth(900)).toEqual({ '--frame-margin': '16px', '--type-say': 'var(--type-body-medium)', '--track-say': '0' });
    expect(atWidth(600)).toEqual({ '--bar-cols': 'auto 1fr auto', '--brand-name-display': 'none' });
  });
});

describe('the earlier tokens', () => {
  const blocks = blocksOf(EARLIER);
  const root = declarationsIn(/^:root \{([^}]*)\}/.exec(EARLIER)?.[1] ?? '');
  const onPaper = blocks.find((block) => block.selector === ':root, [data-ground="paper"]')?.sets ?? {};
  const onFilm = blocks.find((block) => block.selector === '[data-ground="film"]')?.sets ?? {};

  it('stay out of the system’s way: they set none of its names but the two light inks on a film', () => {
    const design = namesSetIn(TOKENS);
    for (const { selector, sets } of blocks) {
      for (const name of Object.keys(sets).filter((each) => design.has(each))) {
        expect(['--ink-2', '--ink-3'], `${name} in ${selector}`).toContain(name);
        expect(selector, name).toBe('[data-ground="film"]');
      }
    }
  });

  it('keep the film hand-over whole: each name from its own on-film value, set beside it', () => {
    expect(Object.keys(onFilm).length).toBeGreaterThan(20);
    for (const name of Object.keys(onFilm).filter((each) => each !== '--focus-ring')) {
      expect(onFilm[name], name).toBe(`var(${name}-film)`);
      expect(root[`${name}-film`], `${name}-film`).toBeDefined();
    }
  });

  it('hand over the same names on paper as on a film, but the two inks the system owns on the page', () => {
    const filmNames = Object.keys(onFilm).filter((name) => name !== '--ink-2' && name !== '--ink-3');
    expect(Object.keys(onPaper).sort()).toEqual(filmNames.sort());
  });
});

// Names set from outside the stylesheet that reads them.
const SET_FROM_OUTSIDE = new Set([
  // Written by script on an element, as geometry: the screens' own, as the head of components.css lists them.
  '--film-zoom', '--k', '--i', '--at', '--p', '--x', '--y', '--s', '--bw', '--bh', '--r', '--cx', '--cy', '--px', '--zx', '--zy', '--zw', '--zh', '--tw', '--th', '--head-y',
  // Read by components.css and never written: the film viewer does the pushing in and the light is drawn in SVG.
  '--z', '--tx', '--ty',
  // Written by the slider.
  '--span',
  // The earlier parts' hooks, which the part round a block sets in its own stylesheet and the block reads with a fallback.
  '--icon-size', '--kbd-ink', '--kbd-fill', '--kbd-edge', '--slider-segment-min',
]);

/** Every stylesheet under app/src, by path. */
function stylesheets(folder: string): string[] {
  return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const path = join(folder, entry.name);
    if (entry.isDirectory()) return stylesheets(path);
    return entry.name.endsWith('.css') ? [path] : [];
  });
}

describe('every stylesheet', () => {
  const source = fileURLToPath(new URL('..', import.meta.url));
  const sheets = stylesheets(source);
  it('leaves the system’s values to the system, except the earlier parts’ two film inks', () => {
    const system = namesSetIn(TOKENS);
    const conflicts: string[] = [];
    for (const path of sheets.filter((each) => !/look-and-feel\/(tokens|components)\.css$/.test(each))) {
      for (const { selector, sets } of blocksOf(meaning(readFileSync(path, 'utf8')))) {
        for (const name of Object.keys(sets).filter((each) => system.has(each))) {
          if (path.endsWith('/earlier-tokens.css') && selector === '[data-ground="film"]' && ['--ink-2', '--ink-3'].includes(name)) continue;
          conflicts.push(`${path.slice(source.length)}: ${selector}: ${name}`);
        }
      }
    }
    expect(conflicts).toEqual([]);
  });
  // The shared sheets, each of which base.css imports for the whole page: the system's tokens, the
  // earlier tokens, the named timings, the choreography's, and confidence.css,
  // which the earlier parts read.
  const shared = new Set(
    sheets
      .filter((path) => /look-and-feel\/(tokens|earlier-tokens|named-timings|confidence)\.css$/.test(path) || /choreography\/[^/]+\.css$/.test(path))
      .flatMap((path) => [...namesSetIn(readFileSync(path, 'utf8'))]),
  );

  it('reads no token nobody sets', () => {
    const unset: string[] = [];
    for (const path of sheets) {
      const css = readFileSync(path, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
      const own = namesSetIn(css);
      for (const [, name = ''] of css.matchAll(/var\(\s*(--[a-z0-9-]+)/g)) {
        if (!shared.has(name) && !own.has(name) && !SET_FROM_OUTSIDE.has(name)) unset.push(`${path.slice(source.length)}: ${name}`);
      }
    }
    expect(unset).toEqual([]);
  });
});
