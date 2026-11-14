import { joinLoop } from '@/choreography/one-loop';
import { takeKeys } from '@/building-blocks/keyboard/keyboard';
import { SYSTEM_ASKS_FOR_REDUCED } from '@/choreography/reduced-motion';
import { durationOf } from '@/choreography/timing';
import type { Stage } from '@/spine-3d/scene/stage';
import type { DrawnSpine } from '@/spine-3d/scene/drawn-spine';
import { REVEAL_RISE } from '@/spine-3d/scene/look';
import { viewBetween } from '@/spine-3d/motion/easing';
import type { FilmDriver } from '../../Stage';
import { LENGTH as MESH_LENGTH, sample } from './timeline';
import { LENGTH as POINTS_LENGTH } from './points-timeline';
import { pointsMove } from './points-move';
export const MOVE_TIMELINE: 'points' | 'mesh' = 'points';
const LENGTHS = { points: POINTS_LENGTH, mesh: MESH_LENGTH };
const LENGTH = LENGTHS[MOVE_TIMELINE];
import './move.css';
export interface Move { set(progress: number): void; play(): void; finish(): void; cancel(): void; dispose(): void }
export interface MoveParts { stage: Stage; spine: DrawnSpine; films: FilmDriver; canvas: HTMLCanvasElement; host: HTMLElement; stageElement: HTMLElement; glow?: HTMLElement; restView?(): import('@/spine-3d/looking/view').View; arrivalParts?(): readonly HTMLElement[]; done(): void }
export function createMove(parts: MoveParts): Move {
  const { stage, spine, films, canvas, host, stageElement } = parts;
  const shown = films.opening(canvas.getBoundingClientRect());
  const from = shown === null ? stage.view() : stage.viewAlong('front', shown) ?? stage.view();
  const to = parts.restView?.() ?? stage.namedView('three-quarter');
  const frozen = stage.film('front')?.plane();
  if (frozen !== undefined) spine.pressOnto({ towardReader: [...frozen.towardReader], out: frozen.out });
  const points = MOVE_TIMELINE === 'points' ? pointsMove(stage, spine, films, canvas, from, to) : null;
  const system = window.matchMedia(SYSTEM_ASKS_FOR_REDUCED);
  const reduced = () => system.matches || host.closest('[data-motion="reduced"]') !== null;
  const fadeHalf = durationOf(host, '--dur-fade');
  let leave: (() => void) | null = null, stopped = false, elapsed = 0;
  const cleanStyles = () => {
    stageElement.style.removeProperty('--move-film-opacity');
    parts.glow?.removeAttribute('data-forming'); parts.glow?.style.removeProperty('--move-glow');
    host.removeAttribute('data-moving'); stageElement.removeAttribute('data-forming');
    host.style.removeProperty('opacity');
    for (const tile of stageElement.querySelectorAll<HTMLElement>('.tile')) { tile.style.removeProperty('--move-mark'); tile.style.removeProperty('opacity'); }
    for (const caption of stageElement.querySelectorAll<HTMLElement>('.caption')) caption.style.removeProperty('opacity');
    for (const part of parts.arrivalParts?.() ?? []) { part.style.removeProperty('opacity'); part.style.removeProperty('transform'); }
  };
  const stop = () => { leave?.(); leave = null; };
  const set = (progress: number, holdFilm = false) => {
    host.setAttribute('data-moving', ''); stageElement.setAttribute('data-forming', '');
    if (points !== null) {
      const found = points.set(progress);
      stage.drawNow();
      if (holdFilm) return;
      films.set(found.trips);
      stageElement.style.setProperty('--move-names', String(found.names));
      for (const tile of stageElement.querySelectorAll<HTMLElement>('.tile[data-place]')) {
        const kind = tile.dataset.place as import('@/shapes').FilmKind;
        tile.style.opacity = found.walls.includes(kind as 'front' | 'side') ? String(found.hand) : '';
        tile.style.setProperty('--move-mark', '1');
      }
      for (const caption of stageElement.querySelectorAll<HTMLElement>('.caption[data-place]')) caption.style.opacity = String(found.caption(caption.dataset.place as import('@/shapes').FilmKind));
      parts.glow?.setAttribute('data-forming', ''); parts.glow?.style.setProperty('--move-glow', String(found.glow));
      (parts.arrivalParts?.() ?? []).forEach((part, index) => { const arrival = found.arrival(index); part.style.opacity = String(arrival.opacity); part.style.transform = `translateY(calc(var(--rise) * ${String(arrival.rise)}))`; });
      return;
    }
    const s = sample(progress);
    films.set(s.trips);
    const view = viewBetween(from, to, s.travel);
    const turn = viewBetween(from, to, s.turn);
    stage.setView({ ...view, azimuth: turn.azimuth, elevation: turn.elevation, perspective: s.depth,
      pxPerMm: [from.pxPerMm[0] * (to.pxPerMm[0] / from.pxPerMm[0]) ** s.travel, from.pxPerMm[1] * (to.pxPerMm[1] / from.pxPerMm[1]) ** s.travel] });
    spine.set({ pose: 1, reveal: 1, fill: 1, strength: 1, tint: 0 });
    spine.setSpine({ depth: s.depth, fill: s.fill, fillFrom: 'top', revealRise: 0, cageOnFilm: 1 - s.ink, shadow: s.shadow, offset: [0, 0, 0], scale: 1, turned: [0, 0] });
    const heights = spine.levels.map((level) => spine.about(level)?.pose.centre[2] ?? 0);
    const high = Math.max(...heights), low = Math.min(...heights);
    spine.levels.forEach((level, i) => {
      const height = heights[i] ?? low;
      const at = high === low ? 0 : (high - height) / (high - low);
      const covered = Math.max(0, Math.min(1, (s.fill - at) * spine.levels.length));
      spine.set({ cage: s.open(i) * (1 - covered) }, level);
    });
    parts.glow?.setAttribute('data-forming', ''); parts.glow?.style.setProperty('--move-glow', String(s.glow));
    (parts.arrivalParts?.() ?? []).forEach((part, index) => { const arrival = s.arrival(index); part.style.opacity = String(arrival.opacity); part.style.transform = `translateY(calc(var(--rise) * ${String(arrival.rise)}))`; });
    stageElement.style.setProperty('--move-names', String(s.names));
    for (const tile of stageElement.querySelectorAll<HTMLElement>('.tile[data-place]')) {
      const kind = tile.dataset.place as import('@/shapes').FilmKind;
      tile.style.setProperty('--move-mark', String(s.marks(kind)));
    }
    for (const caption of stageElement.querySelectorAll<HTMLElement>('.caption[data-place]')) caption.style.opacity = String(s.caption(caption.dataset.place as import('@/shapes').FilmKind));
    stage.drawNow();
  };
  const finish = () => {
    if (stopped) return;
    stopped = true; stop(); points?.clear();
    spine.set({ cage: 0, pose: 1, reveal: 1, fill: 1, strength: 1, tint: 0 });
    spine.setSpine({ depth: 1, fill: 1, revealRise: REVEAL_RISE, cageOnFilm: 0, shadow: 1, offset: [0, 0, 0], turned: [0, 0], scale: 1 });
    stage.setView(() => parts.restView?.() ?? stage.namedView('three-quarter')); films.finish(); cleanStyles(); stage.drawNow(); parts.done();
  };
  const skip = (event: PointerEvent) => {
    if (stopped || event.target instanceof Element && event.target.closest('.steps') !== null) return;
    event.preventDefault(); event.stopPropagation();
    const swallow = (click: MouseEvent) => { click.preventDefault(); click.stopImmediatePropagation(); document.removeEventListener('click', swallow, true); };
    document.addEventListener('click', swallow, { capture: true, once: true });
    finish();
  };
  const unkey = takeKeys('gesture', ['Enter'], (event) => { if (stopped || event.repeat) return false; finish(); return true; });
  document.addEventListener('pointerdown', skip, true);
  const changed = () => { if (!stopped) finish(); };
  window.addEventListener('resize', changed); system.addEventListener('change', changed);
  const preference = new MutationObserver(() => { if (reduced()) changed(); }); preference.observe(host.closest('[data-motion]') ?? document.documentElement, { attributes: true, attributeFilter: ['data-motion'] });
  const cancel = () => { stopped = true; stop(); points?.clear(); cleanStyles(); };
  return { set, finish, cancel, play() {
    if (stopped || leave !== null) return;
    const fading = reduced();
    if (!fading) set(0, points !== null);
    else { host.setAttribute('data-moving', ''); stageElement.setAttribute('data-forming', ''); films.set({ front: 0, side: 0, 'bend-left': 0, 'bend-right': 0 }); host.style.opacity = '0'; }
    let openingDrawn = false, handedOver = points === null;
    leave = joinLoop((dt) => {
      if (!fading && !handedOver) {
        // The first callback paints the opening wall with its DOM film still present.
        // Only the following frame may remove that fallback and start the clock.
        if (!openingDrawn) { openingDrawn = true; set(0, true); return; }
        handedOver = true; set(0); return;
      }
      elapsed += dt;
      if (fading) { for (const part of parts.arrivalParts?.() ?? []) { part.style.opacity = String(Math.max(0, (elapsed - fadeHalf) / fadeHalf)); part.style.transform = 'none'; } if (elapsed >= fadeHalf) { films.finish(); stage.setView(to); stageElement.style.setProperty('--move-mark', '0'); stageElement.style.setProperty('--move-names', '0'); } stageElement.style.setProperty('--move-film-opacity', String(elapsed < fadeHalf ? 1 - elapsed / fadeHalf : (elapsed - fadeHalf) / fadeHalf)); if (elapsed >= fadeHalf * 2) { finish(); return; } host.style.opacity = String(Math.max(0, (elapsed - fadeHalf) / fadeHalf)); }
      else if (elapsed >= LENGTH) finish(); else set(elapsed / LENGTH);
    });
  }, dispose() { cancel(); unkey(); document.removeEventListener('pointerdown', skip, true); window.removeEventListener('resize', changed); system.removeEventListener('change', changed); preference.disconnect(); stageElement.style.removeProperty('--move-names'); } };
}
