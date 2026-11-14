import { joinLoop } from '@/choreography/one-loop';
import { durationOf } from '@/choreography/timing';
import { cameras } from '@/film-viewer/camera/cameras';
import { settle, viewBetween, VIEW_MOVE_MS } from '@/spine-3d/motion/easing';
import type { Spine3DHandle } from '@/spine-3d/view/Spine3D';
import type { FilmDriver } from '../Stage';
import { motionIsReduced } from '../glide-cameras';
export function compareFilm(driver: FilmDriver, handle: Spine3DHandle, canvas: HTMLCanvasElement): () => void {
  const stage = handle.stage();
  const kind = driver.compared;
  if (stage === null || kind === null) return () => undefined;
  const from = stage.view();
  const reduced = motionIsReduced(canvas);
  let elapsed = 0;
  const set = () => {
    const front = stage.namedView('front');
    let target = { ...front, aimAt: [stage.size().width * .75, front.aimAt[1]] as const };
    if (kind === 'front' || kind === 'side') {
      const shown = driver.shown(kind, canvas.getBoundingClientRect());
      if (shown === null) return;
      target = stage.viewAlong(kind, { ...shown, firstPixelAt: [shown.firstPixelAt[0] + driver.separation, shown.firstPixelAt[1]] }) ?? target;
    }
    stage.setView(reduced ? target : viewBetween(from, target, settle(Math.min(1, elapsed / VIEW_MOVE_MS))));
    stage.drawNow();
  };
  set();
  const stopCamera = cameras.subscribe((film) => { if (film === kind) set(); });
  const duration = reduced ? 0 : durationOf(canvas, '--dur-scene');
  const leave = joinLoop((dt) => { elapsed += dt; set(); if (elapsed >= duration) leave(); });
  return () => { leave(); stopCamera(); };
}
