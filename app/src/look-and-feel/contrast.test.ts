// What is read must be readable. This works the WCAG contrast ratio out
// from the tokens themselves, so a colour cannot be changed to one that
// fails without a test saying so.
//
//   - text that must be read is at least 4.5:1 (AA), the main ink 7:1;
//   - a sign or a line that carries meaning is at least 3:1;
//   - on a film, marks are measured over black, where they are drawn, and
//     each against its own casing over pure white, the brightest a film can
//     be (DESIGN.md section 9 works the same figures out);
//   - found and needs a look are told apart as greys too, not by hue alone.
//
// The tokens write some colours with three figures (#fff) and some as
// rgba(); a colour with an alpha is laid over the ground it is named on
// before it is measured.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const css = readFileSync(fileURLToPath(new URL('./tokens.css', import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const ROOT = /:root\s*\{([^}]*)\}/.exec(css)?.[1] ?? '';
const VALUES = Object.fromEntries([...ROOT.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)].map(([, name = '', value = '']) => [name, value.trim()]));

type Rgb = [number, number, number];
interface Colour {
  rgb: Rgb;
  alpha: number;
}

/** A token's colour: #rgb, #rrggbb or rgba(r, g, b, a). */
function colour(name: string): Colour {
  const value = VALUES[name];
  if (value === undefined) throw new Error(`${name} is not set in tokens.css`);
  const short = /^#([0-9a-f])([0-9a-f])([0-9a-f])$/i.exec(value);
  if (short !== null) return { rgb: [1, 2, 3].map((at) => parseInt((short[at] ?? '0').repeat(2), 16)) as Rgb, alpha: 1 };
  const long = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(value);
  if (long !== null) return { rgb: [1, 2, 3].map((at) => parseInt(long[at] ?? '0', 16)) as Rgb, alpha: 1 };
  const rgba = /^rgba\(\s*(\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\s*\)$/.exec(value);
  if (rgba !== null) return { rgb: [1, 2, 3].map((at) => Number(rgba[at])) as Rgb, alpha: Number(rgba[4]) };
  throw new Error(`${name} is not a colour this test reads: "${value}"`);
}

/** A solid colour: `name`, laid over `ground` when it lets some of the ground through. */
function solid(name: string, ground?: string): Rgb {
  const { rgb, alpha } = colour(name);
  if (alpha === 1) return rgb;
  if (ground === undefined) throw new Error(`${name} lets the ground through; name the ground it is on`);
  const under = solid(ground);
  return rgb.map((channel, at) => channel * alpha + (under[at] ?? 0) * (1 - alpha)) as Rgb;
}

/** `name` laid over a colour that is already solid. */
function over(name: string, under: Rgb): Rgb {
  const { rgb, alpha } = colour(name);
  return rgb.map((channel, at) => channel * alpha + (under[at] ?? 0) * (1 - alpha)) as Rgb;
}

/** Relative luminance, as WCAG 2 defines it. */
function luminance(rgb: Rgb): number {
  const [r = 0, g = 0, b = 0] = rgb.map((value) => {
    const channel = value / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(one: Rgb, other: Rgb): number {
  const [light, dark] = [luminance(one), luminance(other)].sort((a, b) => b - a) as [number, number];
  return (light + 0.05) / (dark + 0.05);
}

/** CIE L*: how light a colour is when its hue is taken away. */
function lightness(rgb: Rgb): number {
  const y = luminance(rgb);
  return y > 0.008856 ? 116 * Math.cbrt(y) - 16 : 903.3 * y;
}

const PAGE = solid('--page');
const FILM = solid('--film');

describe('on the page', () => {
  it('is white, and a film’s ground is black', () => {
    expect(PAGE).toEqual([255, 255, 255]);
    expect(FILM).toEqual([0, 0, 0]);
  });

  it('sets the main ink at 7:1 or more, on the page and at its foot', () => {
    expect(contrast(solid('--ink'), PAGE)).toBeGreaterThanOrEqual(7);
    expect(contrast(solid('--ink'), solid('--page-soft'))).toBeGreaterThanOrEqual(7);
  });

  it('sets the second ink at 4.5:1 or more on every ground it is read on', () => {
    const grounds: Rgb[] = [
      PAGE,
      solid('--page-soft'),
      solid('--page-wash-1'),
      solid('--page-wash-2'),
      solid('--glow'),
      solid('--fill', '--page'),
      solid('--track', '--page'),
      solid('--glass-solid'),
    ];
    for (const [at, ground] of grounds.entries()) expect(contrast(solid('--ink-2'), ground), String(at)).toBeGreaterThanOrEqual(4.5);
  });

  it('keeps the third ink at 3:1 or more: it is for a disabled button’s words only, never text that must be read', () => {
    expect(contrast(solid('--ink-3'), PAGE)).toBeGreaterThanOrEqual(3);
  });

  it('sets the words on the primary button at 7:1 or more, under the pointer and pressed too', () => {
    for (const face of ['--ink', '--ink-hover', '--ink-pressed']) expect(contrast(solid('--on-ink'), solid(face)), face).toBeGreaterThanOrEqual(7);
  });

  it('draws the accent at 3:1 or more on the page, for the ring and the meter, and 4.5:1 on a film', () => {
    expect(contrast(solid('--accent'), PAGE)).toBeGreaterThanOrEqual(3);
    expect(contrast(solid('--accent'), FILM)).toBeGreaterThanOrEqual(4.5);
  });

  it('draws pink at 3:1 or more as a mark: on white, and on the thin glass over the glow its marks sit on', () => {
    expect(contrast(solid('--pink'), PAGE)).toBeGreaterThanOrEqual(3);
    expect(contrast(solid('--pink'), solid('--glass-thin', '--glow'))).toBeGreaterThanOrEqual(3);
  });
});

describe('on a film', () => {
  it('draws an outline and its name at 7:1 or more over black, and needs a look at 4.5:1', () => {
    expect(contrast(solid('--mark', '--film'), FILM)).toBeGreaterThanOrEqual(7);
    expect(contrast(solid('--mark-name', '--film'), FILM)).toBeGreaterThanOrEqual(7);
    expect(contrast(solid('--look'), FILM)).toBeGreaterThanOrEqual(4.5);
  });

  it('keeps every line 3:1 against its own casing, and a name 4.3:1, even over pure white', () => {
    const white: Rgb = [255, 255, 255];
    const against = (line: string, casing: string) => {
      const ground = over(casing, white);
      return contrast(over(line, ground), ground);
    };
    expect(against('--mark', '--mark-casing')).toBeGreaterThanOrEqual(3);
    expect(against('--look', '--mark-casing-look')).toBeGreaterThanOrEqual(3);
    expect(against('--accent', '--mark-casing-on')).toBeGreaterThanOrEqual(3);
    // 4.5 needs film no lighter than #f8f8f8; over pure white the name reads 4.3, as DESIGN.md records.
    expect(against('--mark-name', '--mark-name-casing')).toBeGreaterThanOrEqual(4.3);
  });

  it('tells found and needs a look apart as greys: the white outline is at least 15 lighter', () => {
    expect(lightness(solid('--mark', '--film')) - lightness(solid('--look'))).toBeGreaterThanOrEqual(15);
  });

  it('keeps the recorded bright-bone contrast, and readable names through the documented film limit', () => {
    const bone: Rgb = [208, 208, 208];
    for (const [line, casing, ratio] of [
      ['--mark', '--mark-casing', 4.2],
      ['--look', '--mark-casing-look', 4.4],
      ['--accent', '--mark-casing-on', 3.7],
      ['--mark-name', '--mark-name-casing', 5.8],
    ] as const) {
      const ground = over(casing, bone);
      expect(contrast(over(line, ground), ground), line).toBeCloseTo(ratio, 1);
    }
    const lightest: Rgb = [248, 248, 248];
    const ground = over('--mark-name-casing', lightest);
    expect(contrast(over('--mark-name', ground), ground)).toBeGreaterThanOrEqual(4.5);
  });
});
