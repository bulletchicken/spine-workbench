// A canvas's pointer, turned into turning: a drag turns the spine, it
// glides a little after release, the wheel or a pinch zooms, and pointing
// at a bone names its level. Pointer only: no key is listened for.
//
// This is the one place the 3D view moves anything over time by itself, and
// only because a hand threw it. Setting a view from outside ends the glide.

import { joinLoop } from '@/choreography/one-loop';
import { type Bounds, fittedScale } from '../looking/framing';
import { perspectiveFor } from '../looking/named-views';
import { type Turn, makeTurning } from '../looking/turning';
import type { CanvasSize, View } from '../looking/view';
import type { Pointed } from './pointed';
import type { Stage } from './stage';

/** The attribute the canvas wears while a hand may turn it: `free`, `on-a-bone` or `turning`. The stylesheet picks the cursor from it. */
const HAND = 'data-hand';

export interface HandTurningOptions {
  reducedMotion?(): boolean;
  /** The spine was turned or zoomed by hand. */
  onTurned?(): void;
  /** The level under the pointer changed; `null` when it is over no bone. */
  onPoint?(pointed: Pointed | null): void;
  /** A bone was pressed without dragging; `null` for a press on nothing. */
  onChoose?(chosen: Pointed | null): void;
}

export interface HandTurning {
  /** Stop any glide now. */
  rest(): void;
  /** Stop listening. */
  detach(): void;
}

export function attachHandTurning(stage: Stage, canvas: HTMLCanvasElement, options: HandTurningOptions = {}): HandTurning {
  const turning = makeTurning({ azimuth: 0, elevation: 0, zoom: 1 });
  const captured = new Set<number>();
  let gliding: (() => void) | null = null;
  let pointed: Pointed | null = null;
  let lastPointer: PointerEvent | null = null;
  let pointing: (() => void) | null = null;
  /** The view the hand took hold of: its aim is kept, so the spine does not jump to the middle. */
  let held: View | null = null;

  const place = (event: { clientX: number; clientY: number }): [number, number] => {
    const box = canvas.getBoundingClientRect();
    return [event.clientX - box.left, event.clientY - box.top];
  };

  /** Take hold of the view as it stands. */
  const grasp = () => {
    held = stage.view();
    const fitted = fittedScale(stage.bounds(), stage.size());
    turning.set({ azimuth: held.azimuth, elevation: held.elevation, zoom: held.pxPerMm[0] / fitted });
  };

  /** The scale that fits the stage's box to a canvas, kept while the stage gives the same box for a canvas of the same size. */
  let fitted: { bounds: Bounds; width: number; height: number; scale: number } | null = null;
  const fittedTo = (size: CanvasSize) => {
    const bounds = stage.bounds();
    if (fitted?.bounds !== bounds || fitted.width !== size.width || fitted.height !== size.height) fitted = { bounds, width: size.width, height: size.height, scale: fittedScale(bounds, size) };
    return fitted.scale;
  };

  /** The view the hand's turn makes, for a canvas of any size. */
  const viewFor = (turn: Turn, from: View, grasped: CanvasSize) => (size: CanvasSize): View => {
    const scale = fittedTo(size) * turn.zoom;
    return {
      azimuth: turn.azimuth,
      elevation: turn.elevation,
      perspective: perspectiveFor(turn.azimuth, turn.elevation),
      pxPerMm: [scale, scale],
      aim: from.aim,
      // The aim keeps its place relative to the middle of the canvas.
      aimAt: [size.width / 2 + (from.aimAt[0] - grasped.width / 2), size.height / 2 + (from.aimAt[1] - grasped.height / 2)],
    };
  };

  let graspedSize: CanvasSize = stage.size();
  /** The stage's count of views set, as this hand last left it. */
  let mine = -1;
  const show = () => {
    if (held === null) return;
    stage.setView(viewFor(turning.now(), held, graspedSize));
    mine = stage.viewsSet();
    options.onTurned?.();
  };

  const rest = () => {
    gliding?.();
    gliding = null;
    held = null;
  };

  const point = (event: PointerEvent) => {
    pointing?.();
    pointing = null;
    const [x, y] = place(event);
    const now = stage.pick(x, y);
    if (now === null && pointed === null) return;
    if (now !== null && pointed !== null && now.spine === pointed.spine && now.kind === pointed.kind && (now.kind === 'part' && pointed.kind === 'part' ? now.layer === pointed.layer && now.id === pointed.id : now.level === pointed.level)) return;
    pointed = now;
    canvas.setAttribute(HAND, now === null ? 'free' : 'on-a-bone');
    options.onPoint?.(now);
  };
  /**
   * After a frame is drawn, what is under a still pointer is asked again on the next frame, not at once, and not at all if the
   * pointer moves first. The browser sends at most one pointer move a frame, so the stage is asked at most once a frame.
   */
  const pointSoon = () => {
    pointing ??= joinLoop(() => {
      pointing?.();
      pointing = null;
      if (lastPointer !== null && !turning.held()) point(lastPointer);
    });
  };

  const onDown = (event: PointerEvent) => {
    if (event.button !== 0 && event.pointerType === 'mouse') return;
    canvas.setPointerCapture(event.pointerId);
    captured.add(event.pointerId);
    gliding?.();
    gliding = null;
    if (!turning.held()) {
      grasp();
      graspedSize = stage.size();
    }
    const [x, y] = place(event);
    turning.press(event.pointerId, x, y, event.timeStamp);
    canvas.setAttribute(HAND, 'turning');
  };

  const onMove = (event: PointerEvent) => {
    if (!turning.held()) {
      if (event.pointerType !== 'touch') { lastPointer = event; point(event); }
      return;
    }
    const [x, y] = place(event);
    if (turning.move(event.pointerId, x, y, event.timeStamp)) show();
  };

  const onUp = (event: PointerEvent) => {
    if (!turning.held()) return;
    if (canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
    captured.delete(event.pointerId);
    const { tapped } = turning.release(event.pointerId, event.timeStamp);
    if (turning.held()) return;
    canvas.setAttribute(HAND, pointed === null ? 'free' : 'on-a-bone');
    if (tapped) {
      const [x, y] = place(event);
      options.onChoose?.(stage.pick(x, y));
      return;
    }
    if (options.reducedMotion?.()) { rest(); return; }
    gliding = joinLoop((dt) => {
      // A view set from outside ends the glide: the spine is no longer where the hand threw it.
      if (!options.reducedMotion?.() && stage.viewsSet() === mine && turning.glide(dt)) show();
      else {
        gliding?.();
        gliding = null;
      }
    });
  };

  const onCancel = (event: PointerEvent) => {
    turning.release(event.pointerId, event.timeStamp);
    if (canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
    captured.delete(event.pointerId);
    rest();
    canvas.setAttribute(HAND, 'free');
  };

  const onWheel = (event: WheelEvent) => {
    event.preventDefault();
    if (!turning.held() && gliding === null) {
      grasp();
      graspedSize = stage.size();
    }
    turning.wheel(event.deltaY);
    show();
  };

  const onLeave = () => {
    lastPointer = null;
    if (pointed === null) return;
    pointed = null;
    options.onPoint?.(null);
  };

  const stopDrawn = stage.onDrawn(() => { if (lastPointer !== null && !turning.held()) pointSoon(); });
  canvas.setAttribute(HAND, 'free');
  canvas.addEventListener('pointerdown', onDown);
  canvas.addEventListener('pointermove', onMove);
  canvas.addEventListener('pointerup', onUp);
  canvas.addEventListener('pointercancel', onCancel);
  canvas.addEventListener('pointerleave', onLeave);
  canvas.addEventListener('wheel', onWheel, { passive: false });

  return {
    rest,
    detach() {
      stopDrawn();
      rest();
      pointing?.();
      pointing = null;
      for (const pointer of captured) if (canvas.hasPointerCapture(pointer)) canvas.releasePointerCapture(pointer);
      captured.clear();
      canvas.removeEventListener('pointerdown', onDown);
      canvas.removeEventListener('pointermove', onMove);
      canvas.removeEventListener('pointerup', onUp);
      canvas.removeEventListener('pointercancel', onCancel);
      canvas.removeEventListener('pointerleave', onLeave);
      canvas.removeEventListener('wheel', onWheel);
      canvas.removeAttribute(HAND);
    },
  };
}
