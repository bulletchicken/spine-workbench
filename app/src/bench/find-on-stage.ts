import { useLayoutEffect, useMemo, useState } from 'react';
import { useStore } from 'zustand';
import { shownAt } from '@/auto-pass/find-replay';
import { findReplay } from '@/auto-pass/find-replay-clock';
import { useCase } from '@/case-state/case-store';
import { motionMode } from '@/choreography/reduced-motion';
import { durationOnPage, numberOnPage } from '@/choreography/timing';
import { useSystemReducedMotion } from '@/choreography/use-system-reduced-motion';
import { fromFilmMarks } from '@/marks/from-film-marks';
import type { ReplaySignal } from '@/marks/TravellingLight';
import { needsALook } from '@/review/needs-a-look';
import type { FilmKind, Level, Px } from '@/shapes';
import { useCurrentStep } from '@/view-state/current-step';
import { useFindState } from '@/view-state/find-state';
import { usePreferences } from '@/view-state/preferences';
import { selection, useSelection } from '@/view-state/selection';
import { MARK_WORDS } from '@/words/marks';
import type { StageFilm } from './Stage';

type FindFilm = Pick<StageFilm, 'outlines' | 'names' | 'on' | 'only' | 'frame' | 'light'>;

export function useFindOnStage(): {
  films: Partial<Record<FilmKind, FindFilm>>;
  marked: boolean;
  reviewing: boolean;
  replay: ReplaySignal | null;
  onPoint?: (film: FilmKind, key: string | null) => void;
} {
  const marks = useCase((state) => state.case.marks);
  const find = useFindState();
  const { slug } = useCurrentStep();
  const { hot } = useSelection();
  const system = useSystemReducedMotion();
  const reduced = motionMode(system, usePreferences().motion === 'reduced') === 'reduced';
  const plan = useStore(findReplay, (state) => state.plan);
  const revealed = useStore(findReplay, (state) => state.revealed);
  const allShown = useStore(findReplay, (state) => state.allShown);
  const [framedPlan, setFramedPlan] = useState<typeof plan>(null);
  const fadingToSpine = slug === 'find' && reduced && find.phase === 'replaying' && plan !== framedPlan;
  useLayoutEffect(() => {
    if (!fadingToSpine) return;
    const tiles = document.querySelector('.frame > .stage > .tiles');
    tiles?.classList.add('fading');
    const timer = setTimeout(() => { setFramedPlan(plan); }, durationOnPage('--dur-fade'));
    return () => { clearTimeout(timer); tiles?.classList.remove('fading'); };
  }, [fadingToSpine, plan]);
  const measures = useMemo(() => ({ large: numberOnPage('--zoom-find'), small: numberOnPage('--zoom-find-small'), review: numberOnPage('--zoom-review') }), []);
  const level = find.review === null ? undefined : find.reviewing[find.review];
  const reviewing = level !== undefined;
  const hotLevel: Level | undefined = hot === null ? undefined : marks.find((film) => film.film === hot.film)?.vertebrae.find((v) => v.id === hot.vertebra)?.level;
  const films = useMemo(() => {
    const clock = findReplay.getState();
    const shown = plan === null || revealed === 0 ? new Set<string>() : shownAt(plan, clock.times, clock.time).shown;
    const entries = marks.map((film): [FilmKind, FindFilm] => {
      const names = film.film === 'front' || film.film === 'side';
      const outlines = fromFilmMarks(film, (v) => {
        const look = needsALook(v) && !find.looked.includes(v.level) ? 'look' : 'found';
        return { look, spoken: MARK_WORDS.outline(v.level, look === 'look') };
      }, (v) => find.phase === 'found' || (find.phase === 'replaying' && shown.has(`${film.film}:${String(v.id)}`)));
      const focus = outlines.find((m) => m.name === level);
      const points = outlines.flatMap((m) => m.quad);
      const centre: Px | null = focus === undefined ? null : [focus.quad.reduce((sum, p) => sum + p[0], 0) / 4, focus.quad.reduce((sum, p) => sum + p[1], 0) / 4];
      const framing = slug === 'find' && !fadingToSpine && (find.phase === 'found' || find.phase === 'replaying') && points.length > 0;
      return [film.film, {
        outlines, names,
        on: find.phase === 'found' && !reviewing ? outlines.find((m) => m.name === hotLevel)?.key ?? null : null,
        only: reviewing ? focus?.key ?? null : null,
        frame: !framing ? { kind: 'whole' } : reviewing && names && centre !== null ? { kind: 'vertebra', centre, names, over: measures.review } : {
          kind: 'spine', names, most: names ? measures.large : measures.small,
          box: { left: Math.min(...points.map((p) => p[0])), right: Math.max(...points.map((p) => p[0])), top: Math.min(...points.map((p) => p[1])), bottom: Math.max(...points.map((p) => p[1])) },
        },
        light: slug === 'find' && find.phase === 'replaying' && !reduced ? plan?.light[film.film] ?? null : null,
      }];
    });
    return Object.fromEntries(entries);
    // An arrival can be on a partner film only, so this counts all outlines, not just Front's.
  }, [marks, find.looked, find.phase, slug, level, reviewing, hotLevel, measures, reduced, plan, revealed, fadingToSpine]);
  return {
    films, marked: allShown || find.phase === 'found', reviewing,
    replay: slug === 'find' && find.phase === 'replaying' && !reduced ? findReplay.signal : null,
    ...(find.phase === 'found' && !reviewing ? { onPoint: (film: FilmKind, key: string | null) => { selection.getState().hover(key === null ? null : { film, vertebra: Number(key) }); } } : {}),
  };
}
