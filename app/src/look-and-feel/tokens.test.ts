// The tokens exist twice: the app's copy, which CSS reads, and the copy
// beside the design language in docs/design, which people read. The two
// may word a comment differently and must never differ in a value.

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

describe('the design tokens', () => {
  const app = meaning('./tokens.css');
  const docs = meaning('../../../docs/design/tokens.css');

  it('are the same in the app and beside the design language', () => {
    expect(docs).toBe(app);
  });

  it('are really compared: both copies hold the room, the inks and the one breakpoint', () => {
    for (const expected of ['--room-0: #05070a;', '--ink-1: #eef3f9;', '--breathe: 2.6s;', '@media (max-width: 1199px)']) {
      expect(app).toContain(expected);
    }
  });
});
