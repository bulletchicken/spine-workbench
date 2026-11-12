// Reduced motion is asked for in two ways: by the system, through a media
// query, and by the app, through data-motion="reduced" on its root. This
// reads the stylesheets as text and checks that both ways give the same
// answers, and that the answers are the design's:
//
//   - the design system answers the media query in tokens.css and in
//     components.css, and each file gives the same answers under the
//     attribute, rule for rule and value for value;
//   - every distance is nothing and every press and growth is 1, while
//     fades keep their own pace;
//   - every earlier named timing is 1 ms or 0, except how long a toast
//     stays, which is not motion;
//   - the earlier tokens shrink as they always did;
//   - nothing loops (breathe does not run) and nothing travels.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

type Declarations = Record<string, string>;

function declarationsIn(block: string): Declarations {
  return Object.fromEntries([...block.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)].map(([, name = '', value = '']) => [name, value.trim()]));
}

/** The custom properties a stylesheet sets normally, under the media query, and under the attribute. */
function read(path: string): { normally: Declarations; bySystem: Declarations; byApp: Declarations } {
  const css = readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
  const bySystem = /@media \(prefers-reduced-motion: reduce\)\s*\{\s*:root\s*\{([^}]*)\}/.exec(css)?.[1];
  const byApp = /\[data-motion=['"]reduced['"]\]\s*\{([^}]*)\}/.exec(css)?.[1];
  const normally = /(?:^|\})\s*:root\s*\{([^}]*)\}/.exec(css)?.[1];
  if (bySystem === undefined || byApp === undefined || normally === undefined) throw new Error(`${path} has no reduced-motion blocks`);
  return { normally: declarationsIn(normally), bySystem: declarationsIn(bySystem), byApp: declarationsIn(byApp) };
}

/** A stylesheet's text with its comments taken out. */
const textOf = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');

/** The inside of the first `@media` block whose condition is `condition`. */
function insideMedia(css: string, condition: string): string {
  const start = css.indexOf(`@media ${condition}`);
  if (start < 0) throw new Error(`no @media ${condition}`);
  const open = css.indexOf('{', start);
  let depth = 0;
  for (let at = open; at < css.length; at += 1) {
    if (css[at] === '{') depth += 1;
    if (css[at] === '}') depth -= 1;
    if (depth === 0) return css.slice(open + 1, at);
  }
  throw new Error(`@media ${condition} is not closed`);
}

/** Every declaration of every rule, keyed by one selector and its property: "selector | property" to value. */
function declarationsByRule(css: string, dropPrefix = ''): Record<string, string> {
  const found: Record<string, string> = {};
  for (const [, selectors = '', body = ''] of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    for (const selector of selectors.split(',').map((each) => each.trim().replace(/\s+/g, ' '))) {
      const own = selector.startsWith(dropPrefix) ? selector.slice(dropPrefix.length).trim() : `(not under the attribute) ${selector}`;
      for (const declaration of body.split(';').map((each) => each.trim()).filter((each) => each !== '')) {
        const colon = declaration.indexOf(':');
        found[`${own} | ${declaration.slice(0, colon).trim()}`] = declaration.slice(colon + 1).trim().replace(/\s+/g, ' ');
      }
    }
  }
  return found;
}

const REDUCED = '(prefers-reduced-motion: reduce)';
// The system's answers, with its :root written as nothing after the attribute, as the app's file has it.
const BY_SYSTEM = Object.fromEntries(
  Object.entries({ ...declarationsByRule(insideMedia(textOf('../look-and-feel/tokens.css'), REDUCED)), ...declarationsByRule(insideMedia(textOf('../look-and-feel/components.css'), REDUCED)) }).map(([key, value]) => [
    key.replace(/^:root \|/, ' |'),
    value,
  ]),
);
const BY_APP = Object.assign({}, ...['tokens', 'components'].map((name) => {
  const rules = textOf(`../look-and-feel/${name}.css`).match(/\[data-motion="reduced"\][^{}]*\{[^{}]*\}/g) ?? [];
  return declarationsByRule(rules.join('\n'), '[data-motion="reduced"]');
})) as Record<string, string>;
const TOKENS_BY_APP = Object.fromEntries(
  Object.entries(BY_APP)
    .filter(([key]) => key.startsWith(' | '))
    .map(([key, value]) => [key.slice(3), value]),
);

const NAMED = read('../look-and-feel/named-timings.css');
const EARLIER = read('../look-and-feel/earlier-tokens.css');
const BREATHE = read('./breathe.css');
const SETTLE = read('./settle.css');

describe('reduced motion, in the stylesheets', () => {
  it('gives the design system’s answers when the app asks: every rule and value of its two blocks, and nothing more', () => {
    expect(Object.keys(BY_SYSTEM).length).toBeGreaterThan(15);
    expect(BY_APP).toEqual(BY_SYSTEM);
  });

  it('gives the same answers whether the system asks or the app does, in the earlier sheets', () => {
    for (const sheet of [EARLIER, NAMED, BREATHE, SETTLE]) expect(sheet.byApp).toEqual(sheet.bySystem);
  });

  it('makes every named timing 1 ms or 0, except how long a toast stays', () => {
    const names = Object.keys(NAMED.normally);
    expect(names).toContain('--t-refit');
    expect(names).toContain('--t-toast-stay');
    for (const name of names.filter((each) => each !== '--t-toast-stay')) {
      expect(['1ms', '0ms'], name).toContain(NAMED.byApp[name]);
    }
    expect(NAMED.byApp['--t-toast-stay']).toBeUndefined();
    expect(NAMED.normally['--t-toast-stay']).toBe('4600ms');
  });

  it('takes every distance away and every press and growth, and leaves fades their pace', () => {
    expect(TOKENS_BY_APP).toEqual({
      '--rise': '0px',
      '--rise-small': '0px',
      '--drop-note': '0px',
      '--press': '1',
      '--press-seg': '1',
      '--press-badge': '1',
      '--from-ready': '1',
      '--from-note': '1',
      '--from-film': '1',
    });
    for (const key of [...Object.keys(BY_SYSTEM), ...Object.keys(BY_APP)]) {
      expect(key.split(' | ')[1]).not.toMatch(/^--(dur-|delay-|hold-|stagger)/);
    }
  });

  it('shrinks the earlier tokens as it always did', () => {
    expect(EARLIER.byApp).toEqual({
      '--t-quick': '1ms',
      '--t-base': '1ms',
      '--t-calm': '120ms',
      '--t-slow': '160ms',
      '--t-cinema': '160ms',
    });
  });

  it('stops the one loop: breathe does not run', () => {
    expect(BREATHE.normally['--breathing']).toBe('breathe var(--breathe) ease-in-out infinite');
    expect(BREATHE.byApp['--breathing']).toBe('none');
  });

  it('lets nothing travel: a settling line only fades in', () => {
    expect(SETTLE.normally['--settle-rise']).toBe('8px');
    expect(SETTLE.byApp['--settle-rise']).toBe('0px');
  });
});
