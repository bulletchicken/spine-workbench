import { useLayoutEffect, type RefObject } from 'react';
import { joinLoop } from '@/choreography/one-loop';
import { SYSTEM_ASKS_FOR_REDUCED } from '@/choreography/reduced-motion';
import { curveOf, durationOf } from '@/choreography/timing';
import { cameras } from '@/film-viewer/camera/cameras';
import { FILM_ORDER } from './stage-layout';

export function motionIsReduced(part: Element): boolean {
  return part.closest('[data-motion="reduced"]') !== null || window.matchMedia(SYSTEM_ASKS_FOR_REDUCED).matches;
}

/** Cameras join the shared loop only while one has somewhere to travel. */
export function useGlidingCameras(root: RefObject<Element | null>): void {
  useLayoutEffect(() => {
    let leave: (() => void) | null = null;
    const start = () => {
      const part = root.current;
      if (part === null || leave !== null) return;
      const duration = motionIsReduced(part) ? 0 : durationOf(part, '--dur-push');
      const curve = curveOf(part, '--ease-settle');
      leave = joinLoop((dt) => {
        if (cameras.step(dt, duration, curve)) return;
        leave?.();
        leave = null;
      });
    };
    const stop = cameras.subscribe((_kind, moving) => {
      if (moving) start();
    });
    // A child's camera can start before this parent's layout effect runs.
    if (FILM_ORDER.some((kind) => cameras.isGliding(kind))) start();
    return () => {
      stop();
      leave?.();
    };
  }, [root]);
}
