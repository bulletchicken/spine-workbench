// `flash` moves fill, stroke and stroke-width, and all three inherit. A
// shape that leaves its stroke unsaid takes whatever stroke is around it,
// so inside a flashing part it would be drawn with a broad white line. The
// glyphs and the brand mark are drawn inside other parts, shape by shape,
// so every rule that paints one of their shapes says both fill and stroke.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

/** Each rule of a stylesheet, as its selector and the properties it sets. */
function rulesIn(path: string): { selector: string; sets: string[] }[] {
  const css = readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
  return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map(([, selector = '', body = '']) => ({
    selector: selector.trim(),
    sets: [...body.matchAll(/([a-z-]+)\s*:/g)].map(([, property = '']) => property),
  }));
}

describe.each(['./ConfidenceGlyph.css'])('%s', (path) => {
  const painting = rulesIn(path).filter(({ sets }) => sets.includes('fill') || sets.includes('stroke'));

  it('paints its shapes', () => {
    expect(painting.length).toBeGreaterThan(0);
  });

  it('says both fill and stroke wherever it says either', () => {
    for (const { selector, sets } of painting) {
      expect(sets, selector).toContain('fill');
      expect(sets, selector).toContain('stroke');
    }
  });
});
