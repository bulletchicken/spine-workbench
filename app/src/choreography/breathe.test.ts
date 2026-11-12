// The breath is a number and nothing else. breathe.css moves `--breath`
// between 0 and 1 and names no colour: the ring that breathes is drawn in
// a white the tokens give, and nothing glows. This reads the stylesheet as
// text and checks that it holds no colour and no shadow of its own, that
// its loop is the tokens' loop, and that a part at rest holds half a
// breath.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

function sheet(path: string): string {
  return readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
}

describe('the breath', () => {
  const css = sheet('./breathe.css');
  const tokens = sheet('../look-and-feel/earlier-tokens.css');

  it('names no colour and casts no shadow: it is only a number', () => {
    expect(css).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(css).not.toMatch(/\b(?:rgba?|hsla?|color-mix|oklch|oklab)\(/);
    expect(css).not.toMatch(/shadow|filter/);
    expect([...css.matchAll(/(--[a-z-]+)\s*:/g)].map(([, name]) => name)).toEqual(['--breathing', '--breathing', '--breathing', '--breath', '--breath']);
  });

  it('loops for as long as the tokens say', () => {
    expect(css).toMatch(/--breathing:\s*breathe var\(--breathe\) ease-in-out infinite;/);
    expect(tokens).toMatch(/--breathe:\s*2\.6s;/);
  });

  it('rests at half a breath, and does not pass from a part to what is inside it', () => {
    const property = /@property --breath\s*\{([^}]*)\}/.exec(css)?.[1] ?? '';
    expect(property).toMatch(/initial-value:\s*0\.5;/);
    expect(property).toMatch(/inherits:\s*false;/);
  });
});
