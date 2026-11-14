import { cubicBezier } from '@/choreography/curve';
import type { FilmKind } from '@/shapes';
export const LENGTH = 3100;
const E = cubicBezier(.4, 0, .6, 1), G = cubicBezier(.5, 0, .12, 1), T = cubicBezier(.3, 0, .12, 1), L = cubicBezier(.45, 0, .15, 1), P = cubicBezier(.25, 0, .7, 1), S = cubicBezier(.22, 1, .36, 1);
const clamp = (p: number) => Math.max(0, Math.min(1, p));
const span = (t: number, a: number, b: number) => clamp((t - a) / (b - a));
export function sample(progress: number) {
  const t = LENGTH * (Number.isFinite(progress) ? clamp(progress) : 0);
  const departures: Record<FilmKind, number> = { front: 860, side: 710, 'bend-left': 660, 'bend-right': 660 };
  const trips = Object.fromEntries(Object.entries(departures).map(([kind, start]) => [kind, L(span(t, start, start + 900))])) as Record<FilmKind, number>;
  return { t, trips, travel: G(span(t, 860, 1860)), turn: T(span(t, 860, 2360)), depth: E(span(t, 1080, 1800)), fill: P(span(t, 1800, 2900)), shadow: E(span(t, 2460, 3060)), glow: E(span(t, 1700, 2900)), ink: span(t, 860, 1760), names: 1 - span(t, 300, 620),
    open: (i: number) => E(span(t, 30 + 26 * i, 350 + 26 * i)),
    caption: (kind: FilmKind) => 1 - span(t, departures[kind], departures[kind] + 180),
    marks: (kind: FilmKind) => 1 - span(t, departures[kind] + 72, departures[kind] + 450),
    arrival: (i: number) => { const a = Math.min(2760, 2460 + 50 * Math.max(0, i - 2)); const p = span(t, a, a + 340); return { opacity: E(p), rise: 1 - S(p) }; },
  };
}
