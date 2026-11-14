import type { FilmKind, FilmMarks } from '@/shapes';
import type { FilmSize } from '@/film-viewer/film-space';
import { fromFilmMarks } from '@/marks/from-film-marks';

export interface ReplayTimes { lead: number; travel: number; hold: number }
export interface LightMeasures { height: number; edge: number; filmHeight: number }
export interface RevealPlan {
  at: ReadonlyMap<string, number>;
  counted: readonly string[];
  last: number;
  light: Partial<Record<FilmKind, { from: number; to: number; height: number }>>;
}

/** The answer alone sets the order; no kind of watch frame is needed. */
export function revealPlan(marks: readonly FilmMarks[], sizes: Partial<Record<FilmKind, FilmSize>>, light: LightMeasures): RevealPlan | null {
  const films = marks.map((film) => ({
    kind: film.film,
    outlines: fromFilmMarks(film, () => ({ look: 'found', spoken: '' }), () => false)
      .map((m) => ({ key: `${film.film}:${m.key}`, name: m.name, y: m.quad.reduce((sum, p) => sum + p[1], 0) / 4 })),
  }));
  const lead = films.find((film) => film.kind === 'front' && film.outlines.length > 0) ?? films.find((film) => film.kind === 'side' && film.outlines.length > 0);
  if (lead === undefined || lead.outlines.length === 0) return null;
  const at = new Map<string, number>();
  const runs: RevealPlan['light'] = {};
  const shares = new Map<string, number>();
  for (const film of films) {
    if (film.outlines.length === 0) continue;
    const scale = (sizes[film.kind]?.height ?? light.filmHeight) / light.filmHeight;
    const ys = film.outlines.map((m) => m.y);
    const from = Math.min(...ys) - light.edge * scale;
    const to = Math.max(...ys) + light.edge * scale;
    runs[film.kind] = { from, to, height: light.height * scale };
    if (film === lead) for (const m of film.outlines) shares.set(m.name, to === from ? 0.5 : (m.y - from) / (to - from));
  }
  for (const film of films) {
    const run = runs[film.kind];
    if (run === undefined) continue;
    for (const m of film.outlines) at.set(m.key, shares.get(m.name) ?? (run.to === run.from ? 0.5 : (m.y - run.from) / (run.to - run.from)));
  }
  return { at, counted: lead.outlines.map((m) => m.key), last: Math.max(...at.values()), light: runs };
}

export function lightShareAt(times: ReplayTimes, time: number): number | null {
  return time < times.lead || time > times.lead + times.travel ? null : (time - times.lead) / times.travel;
}

export function shownAt(plan: RevealPlan, times: ReplayTimes, time: number): { shown: ReadonlySet<string>; count: number; allShown: boolean } {
  const share = (time - times.lead) / times.travel;
  const shown = new Set([...plan.at].filter(([, at]) => time >= times.lead && at <= share).map(([key]) => key));
  return { shown, count: plan.counted.filter((key) => shown.has(key)).length, allShown: shown.size === plan.at.size };
}

export function replayEnds(plan: RevealPlan, times: ReplayTimes): number { return times.lead + times.travel * plan.last + times.hold; }
