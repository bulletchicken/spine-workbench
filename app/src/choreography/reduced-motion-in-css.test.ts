// Reduced motion is asked for in two ways: by the system, through a media
// query, and by the app, through data-motion="reduced" on its root. This
// reads the stylesheets as text and checks that both ways give the same
// answers, and that the answers are the design's:
//
//   - every named timing is 1 ms or 0, except how long a toast stays,
//     which is not motion;
//   - the tokens shrink to the values the design gives;
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

const NAMED = read('../look-and-feel/named-timings.css');
const TOKENS = read('../look-and-feel/tokens.css');
const BREATHE = read('./breathe.css');
const SETTLE = read('./settle.css');

describe('reduced motion, in the stylesheets', () => {
  it('gives the same answers whether the system asks or the app does', () => {
    for (const sheet of [NAMED, TOKENS, BREATHE, SETTLE]) expect(sheet.byApp).toEqual(sheet.bySystem);
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

  it('shrinks the tokens to the design’s reduced values', () => {
    expect(TOKENS.byApp).toEqual({
      '--t-quick': '1ms',
      '--t-base': '1ms',
      '--t-calm': '120ms',
      '--t-slow': '160ms',
      '--t-cinema': '160ms',
      '--stagger': '0ms',
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
