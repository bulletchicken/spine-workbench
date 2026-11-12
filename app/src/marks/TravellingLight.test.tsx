// The light of the find, drawn through a real layer slot and layer stack.
// What is checked is where the rect stands and how strong it is at each
// share of its journey, and that its stops are the design's.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { act } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { find, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { filmSpace } from '@/film-viewer/film-space';
import { LayerStack } from '@/film-viewer/LayerStack';
import { useLayer } from '@/film-viewer/layer-slot';
import { LIGHT_STOPS, MASK_STOPS, TravellingLight, type ReplaySignal } from './TravellingLight';

const read = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8');
const TOKENS = read('../look-and-feel/tokens.css');

/** A plain number token as tokens.css sets it. */
function tokenNumber(name: string): number {
  const value = new RegExp(`${name}:\\s*([0-9.]+);`).exec(TOKENS)?.[1];
  if (value === undefined) throw new Error(`tokens.css sets no number ${name}`);
  return Number(value);
}

// jsdom does not load the stylesheets, so a number is read where a browser would find it: tokens.css.
vi.mock('@/choreography/timing', async (original) => ({
  ...(await original<typeof import('@/choreography/timing')>()),
  numberOf: (_part: Element, name: string) => tokenNumber(name),
}));

const LIGHT_IN = tokenNumber('--light-in');
const LIGHT_OUT = tokenNumber('--light-out');

const space = filmSpace({ size: { width: 400, height: 800 }, view: { quarterTurns: 0, mirrored: false }, zoom: 1, pan: [0, 0] });

/** A signal the test moves by hand. */
function handSignal(): ReplaySignal & { go(share: number | null): void } {
  let now: number | null = null;
  const listeners = new Set<() => void>();
  return {
    share: () => now,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    go(share) {
      now = share;
      act(() => {
        for (const listener of listeners) listener();
      });
    },
  };
}

function Drawn({ signal }: { signal: ReplaySignal }) {
  const layer = useLayer('light', TravellingLight, { from: 100, to: 700, height: 200, width: 400, signal });
  return <LayerStack space={space} layers={[layer]} />;
}

describe('the travelling light', () => {
  it('stands at its share of the journey, faded in and out at the ends, and is not drawn without a share', () => {
    const signal = handSignal();
    putOnThePage(<Drawn signal={signal} />);
    const band = find('rect.travelling-light');
    expect(band.getAttribute('display')).toBe('none');
    expect(band.getAttribute('width')).toBe('400');
    expect(band.getAttribute('height')).toBe('200');

    // y is from + (to - from) * share - height / 2. It has faded in by --light-in and starts to fade at --light-out.
    expect([LIGHT_IN, LIGHT_OUT]).toEqual([0.05, 0.93]);
    const seen = [0, LIGHT_IN, 0.5, LIGHT_OUT, 1].map((share) => {
      signal.go(share);
      return [Number(band.getAttribute('y')), Number(band.getAttribute('opacity')), band.getAttribute('display')];
    });
    expect(seen.map(([y]) => y)).toEqual([0, 30, 300, 558, 600]);
    const opacities = seen.map(([, opacity]) => opacity as number);
    expect(opacities[0]).toBe(0);
    expect(opacities[1]).toBeCloseTo(1, 9);
    expect(opacities[2]).toBe(1);
    expect(opacities[3]).toBeCloseTo(1, 9);
    expect(opacities[4]).toBe(0);
    expect(seen.every(([, , display]) => display === null)).toBe(true);

    signal.go(null);
    expect(band.getAttribute('display')).toBe('none');
  });

  it('is hidden from assistive technology and its gradients are its own', () => {
    putOnThePage(<Drawn signal={handSignal()} />);
    const band = find('rect.travelling-light');
    expect(band.parentElement?.getAttribute('aria-hidden')).toBe('true');
    const light = band.closest('svg.light');
    expect(light?.getAttribute('viewBox')).toBe('0 0 400 800');
    expect(light?.getAttribute('preserveAspectRatio')).toBe('none');
    expect(light?.getAttribute('style')).toBe('width: 400px; height: 800px;');
    expect(light?.parentElement?.getAttribute('data-layer')).toBe('light');
    const fill = /^url\(#(.+)\)$/.exec(band.getAttribute('fill') ?? '')?.[1];
    const mask = /^url\(#(.+)\)$/.exec(band.getAttribute('mask') ?? '')?.[1];
    expect(document.getElementById(fill ?? '')?.tagName).toBe('linearGradient');
    expect(document.getElementById(mask ?? '')?.tagName).toBe('mask');
  });
});

// ---- The stops, against the tokens ----

/** The arguments of a token's linear-gradient, split at the commas between them. */
function gradientOf(name: string): string[] {
  const value = new RegExp(`${name}:\\s*linear-gradient\\((.*)\\);`).exec(TOKENS)?.[1];
  if (value === undefined) throw new Error(`tokens.css sets no gradient ${name}`);
  const parts: string[] = [];
  let depth = 0;
  let part = '';
  for (const character of value) {
    if (character === '(') depth += 1;
    if (character === ')') depth -= 1;
    if (character === ',' && depth === 0) {
      parts.push(part.trim());
      part = '';
    } else part += character;
  }
  parts.push(part.trim());
  return parts.filter((stop) => !stop.startsWith('to '));
}

/** A colour stop as its colour, its alpha and its offset (the ends left unsaid are 0 and 1). */
function stopsOf(name: string): { colour: string; offset: number; alpha: number }[] {
  const stops = gradientOf(name);
  return stops.map((stop, i) => {
    const [, colour = '', percent] = /^(.*?\)|#[0-9a-f]+|transparent)\s*(?:([\d.]+)%)?$/i.exec(stop) ?? [];
    const rgba = /rgba\((\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\)/.exec(colour);
    const alpha = colour === 'transparent' ? 0 : rgba === null ? 1 : Number(rgba[4]);
    const base = colour === 'transparent' ? 'none' : rgba === null ? colour : `${rgba[1] ?? ''},${rgba[2] ?? ''},${rgba[3] ?? ''}`;
    const offset = percent === undefined ? (i === 0 ? 0 : 1) : Number(percent) / 100;
    return { colour: base, offset, alpha };
  });
}

describe('the light’s stops', () => {
  it('are --find-light’s: white at its alphas, at its offsets', () => {
    const stops = stopsOf('--find-light');
    expect(stops.every((stop) => stop.colour === '255,255,255')).toBe(true);
    expect(stops.map(({ offset, alpha }) => [offset, alpha])).toEqual(LIGHT_STOPS.map(([offset, alpha]) => [offset, alpha]));
  });

  it('are --find-light-mask’s: solid black across the middle, clear at both sides', () => {
    const stops = stopsOf('--find-light-mask');
    expect(stops.filter((stop) => stop.alpha > 0).every((stop) => stop.colour === '#000')).toBe(true);
    expect(stops.map(({ offset, alpha }) => [offset, alpha])).toEqual(MASK_STOPS.map(([offset, alpha]) => [offset, alpha]));
  });

  it('are drawn as written', () => {
    putOnThePage(<Drawn signal={handSignal()} />);
    const gradients = [...document.querySelectorAll('linearGradient')];
    const drawn = gradients.map((gradient) => [...gradient.querySelectorAll('stop')].map((stop) => [Number(stop.getAttribute('offset')), Number(stop.getAttribute('stop-opacity'))]));
    expect(drawn).toEqual([LIGHT_STOPS, MASK_STOPS].map((stops) => stops.map(([offset, alpha]) => [offset, alpha])));
  });
});
