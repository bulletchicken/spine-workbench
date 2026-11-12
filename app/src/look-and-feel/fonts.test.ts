// The typefaces come from the two font packages the app installs and
// from nowhere else. This reads fonts.css as text and checks every source
// it names, since the build's own scan sees fonts only once a screen
// imports the stylesheet.

import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const css = readFileSync(fileURLToPath(new URL('./fonts.css', import.meta.url)), 'utf8');
const tokens = readFileSync(fileURLToPath(new URL('./tokens.css', import.meta.url)), 'utf8');

const faces = [...css.matchAll(/@font-face\s*\{([^}]*)\}/g)].map(([, body = '']) => ({
  family: /font-family:\s*'([^']+)'/.exec(body)?.[1],
  sources: [...body.matchAll(/url\(\s*'?([^')]+)'?\s*\)/g)].map(([, source = '']) => source),
  formats: [...body.matchAll(/format\(\s*'([^']+)'\s*\)/g)].map(([, format = '']) => format),
}));

const PACKAGES = ['@fontsource-variable/geist/', '@fontsource-variable/geist-mono/'];

describe('fonts.css', () => {
  it('declares the two families under the names the tokens use', () => {
    expect(new Set(faces.map((face) => face.family))).toEqual(new Set(['Geist', 'Geist Mono']));
    expect(/--font-sans:\s*Geist,/.test(tokens)).toBe(true);
    expect(/--font-mono:\s*"Geist Mono",/.test(tokens)).toBe(true);
  });

  it('gives every face exactly one source, a woff2 file', () => {
    expect(faces.length).toBeGreaterThan(0);
    for (const face of faces) {
      expect(face.sources, face.family).toHaveLength(1);
      expect(face.sources[0], face.family).toMatch(/\.woff2$/);
      expect(face.formats, face.family).toEqual(['woff2']);
    }
  });

  it('takes every file from an installed font package, and the file is there', () => {
    for (const source of faces.flatMap((face) => face.sources)) {
      expect(
        PACKAGES.some((name) => source.startsWith(`${name}files/`)),
        source,
      ).toBe(true);
      expect(existsSync(fileURLToPath(new URL(`../../node_modules/${source}`, import.meta.url))), source).toBe(true);
    }
  });

  it('names no address: nothing with a scheme, nothing from another origin', () => {
    expect(css).not.toMatch(/[a-z][a-z0-9+.-]*:\/\//i);
    expect(css).not.toMatch(/url\(\s*['"]?(?:\/\/|data:)/i);
    expect(css).not.toMatch(/@import/);
  });

  it('ships both ranges of each family, Latin and extended Latin, and no other file', () => {
    const cuts = faces.map((face) => face.sources[0]?.split('/').pop());
    expect(cuts.sort()).toEqual(
      [
        'geist-latin-ext-wght-normal.woff2',
        'geist-latin-wght-normal.woff2',
        'geist-mono-latin-ext-wght-normal.woff2',
        'geist-mono-latin-wght-normal.woff2',
      ].sort(),
    );
  });
});
