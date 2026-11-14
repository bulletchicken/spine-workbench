import { cubicBezier } from '@/choreography/curve';
import type { FilmKind, Level } from '@/shapes';
import type { FormationScript } from '@/spine-3d/scene/formation-drawing';
export const LENGTH = 4020;
export const ORDER: readonly Level[] = ['T9', 'T3', 'L2', 'T6', 'T12', 'T1', 'L4', 'T8', 'T4', 'L1', 'T10', 'L5', 'T2', 'T7', 'L3', 'T5', 'T11'];
const E = cubicBezier(.4, 0, .6, 1), S = cubicBezier(.22, 1, .36, 1);
const clamp = (p: number) => Math.max(0, Math.min(1, p));
export const span = (t: number, a: number, b: number) => clamp((t - a) / (b - a));
export function scriptFor(level: Level): FormationScript {
  const order = Math.max(0, ORDER.indexOf(level));
  return { begins: 660 + order * 66, fills: 2520 + order * 24, first: 185, spread: [230, 520], loose: 26, lines: [520, 320], fade: [100, 260], reach: 200, hold: 300, threadFade: 220, pointFade: 110, pointSettle: 260, nodeSettle: 180 };
}
export function sample(progress: number) {
  const t = LENGTH * (Number.isFinite(progress) ? clamp(progress) : 0);
  const departures: Record<FilmKind, number> = { front: 3200, side: 3260, 'bend-left': 0, 'bend-right': 30 };
  return { t, stand: E(span(t, 0, 760)), swing: E(span(t, 0, 640)), drift: S(span(t, 440, 3600)), settle: E(span(t, 3140, 4000)), shadow: S(span(t, 3440, 3960)), glow: E(span(t, 2900, 4000)), names: 1 - span(t, 0, 220),
    trips: Object.fromEntries(Object.entries(departures).map(([kind, start]) => [kind, E(span(t, start, start + (kind.startsWith('bend') ? 640 : 720)))])) as Record<FilmKind, number>,
    hand: E(span(t, 3870, 4010)), ownParts: t < LENGTH,
    bone: (level: Level) => { const script = scriptFor(level), age = t - script.begins; return { revealed: span(age, 185, 630), lines: span(age, 520, 840), fill: span(t, script.fills, script.fills + 440), drawing: 1 - span(t, script.fills + 100, script.fills + 360), threads: (age > 0 ? 1 : 0) * (1 - span(age, 300, 520)), lift: (0.26 * S(span(age, 0, 120)) * (1 - S(span(age, 220, 700))) + .09 * S(span(age, 220, 700))) * (1 - S(span(t, 3200, 3520))) }; },
    caption: (kind: FilmKind) => kind.startsWith('bend') ? 1 : 1 - span(t, 0, 220),
    arrival: (i: number) => { const p = span(t, 3580 + Math.max(0, i - 2) * 60, 3860 + Math.max(0, i - 2) * 60); return { opacity: E(p), rise: 1 - S(p) }; },
  };
}
