// The breathing light has one colour, the blue of the design's lumen glow.
// The tokens give that blue with a fixed strength; breathe.css gives it
// again as bare channels, so a lit part can set a strength that follows the
// breath. This reads both stylesheets as text and checks they are one blue.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

function sheet(path: string): string {
  return readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
}

describe('the breathing light', () => {
  const glow = /--lumen-glow:\s*rgba\(\s*(\d+),\s*(\d+),\s*(\d+),/.exec(sheet('../look-and-feel/tokens.css'));
  const light = /--breath-light:\s*([^;]+);/.exec(sheet('./breathe.css'))?.[1];

  it('is the blue of the lumen glow, read from the tokens', () => {
    expect(glow?.slice(1, 4)).toEqual(['110', '190', '255']);
    expect(light?.trim().split(/\s+/)).toEqual(glow?.slice(1, 4));
  });
});
