// What is read must be readable, on both grounds. This works the WCAG
// contrast ratio out from the tokens themselves, so a colour cannot be
// changed to one that fails without a test saying so.
//
//   - text is at least 4.5:1 (AA);
//   - a sign or a line that carries meaning is at least 3:1;
//   - the four confidence states are held to the text figure on both
//     grounds, because their glyphs are small;
//   - the states are told apart as greys too: on each ground their
//     lightness (CIE L*) is in a fixed order, with a step between each.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const css = readFileSync(fileURLToPath(new URL('./tokens.css', import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const HEX = Object.fromEntries([...css.matchAll(/(--[a-z0-9-]+)\s*:\s*(#[0-9a-f]{6})\s*;/gi)].map(([, name = '', hex = '']) => [name, hex]));

function hex(name: string): string {
  const value = HEX[name];
  if (value === undefined) throw new Error(`${name} is not a six-figure hex colour in tokens.css`);
  return value;
}

/** Relative luminance, as WCAG 2 defines it. */
function luminance(colour: string): number {
  const [r = 0, g = 0, b = 0] = [1, 3, 5].map((at) => {
    const channel = parseInt(colour.slice(at, at + 2), 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(one: string, other: string): number {
  const [light, dark] = [luminance(one), luminance(other)].sort((a, b) => b - a) as [number, number];
  return (light + 0.05) / (dark + 0.05);
}

/** CIE L*: how light a colour is when its hue is taken away. */
function lightness(colour: string): number {
  const y = luminance(colour);
  return y > 0.008856 ? 116 * Math.cbrt(y) - 16 : 903.3 * y;
}

const PAPER = hex('--room-0');
const WELL = hex('--well');
const STATES = ['sure', 'look', 'needs', 'by-you'] as const;

describe('on paper', () => {
  it('is white, with a film’s well black', () => {
    expect(PAPER).toBe('#ffffff');
    expect(WELL).toBe('#000000');
  });

  it('sets text at 4.5:1 or more, on the page and on a row under the pointer', () => {
    for (const ink of ['--ink-1-paper', '--ink-2-paper', '--ink-3-paper']) {
      expect(contrast(hex(ink), PAPER), ink).toBeGreaterThanOrEqual(4.5);
      expect(contrast(hex(ink), hex('--room-3')), `${ink} on --room-3`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it('keeps the faintest ink and a control’s outline at 3:1 or more', () => {
    expect(contrast(hex('--ink-4-paper'), PAPER)).toBeGreaterThanOrEqual(3);
    expect(contrast(hex('--line-3-paper'), PAPER)).toBeGreaterThanOrEqual(3);
  });

  it('sets the words on the ink button at 7:1 or more, under the pointer too', () => {
    expect(contrast(hex('--on-lumen-paper'), hex('--lumen-paper'))).toBeGreaterThanOrEqual(7);
    expect(contrast(hex('--on-lumen-paper'), hex('--lumen-hi-paper'))).toBeGreaterThanOrEqual(7);
  });

  it('draws the four confidence states at 4.5:1 or more, and inferred at 3:1', () => {
    for (const state of STATES) expect(contrast(hex(`--${state}-paper`), PAPER), state).toBeGreaterThanOrEqual(4.5);
    expect(contrast(hex('--inferred-paper'), PAPER)).toBeGreaterThanOrEqual(3);
  });

  it('tells the states apart as greys: set by you darkest, then needs, sure, look, inferred', () => {
    const greys = ['by-you', 'needs', 'sure', 'look', 'inferred'].map((state) => lightness(hex(`--${state}-paper`)));
    for (let at = 1; at < greys.length; at += 1) expect((greys[at] ?? 0) - (greys[at - 1] ?? 0), String(at)).toBeGreaterThanOrEqual(4);
  });
});

describe('on a film', () => {
  it('sets text at 4.5:1 or more against the well', () => {
    for (const ink of ['--ink-1-film', '--ink-2-film', '--ink-3-film']) expect(contrast(hex(ink), WELL), ink).toBeGreaterThanOrEqual(4.5);
  });

  it('draws every confidence state at 4.5:1 or more against the well, where its casing puts it', () => {
    for (const state of [...STATES, 'inferred']) expect(contrast(hex(`--${state}-film`), WELL), state).toBeGreaterThanOrEqual(4.5);
  });

  it('sets the letter on a white badge at 7:1 or more', () => {
    expect(contrast(hex('--on-lumen-film'), hex('--lumen-film'))).toBeGreaterThanOrEqual(7);
  });

  it('tells the states apart as greys: needs darkest, then inferred, sure, look, set by you', () => {
    const greys = ['needs', 'inferred', 'sure', 'look', 'by-you'].map((state) => lightness(hex(`--${state}-film`)));
    for (let at = 1; at < greys.length; at += 1) expect((greys[at] ?? 0) - (greys[at - 1] ?? 0), String(at)).toBeGreaterThanOrEqual(3);
  });
});
