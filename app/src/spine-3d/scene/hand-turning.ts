// A canvas's pointer, turned into turning: a drag turns the spine, it
// glides a little after release, the wheel or a pinch zooms, and pointing
// at a bone names its level. Pointer only: no key is listened for.
//
// This is the one place the 3D view moves anything over time by itself, and
// only because a hand threw it. Setting a view from outside ends the glide.

import type { Level } from '@/shapes';
import { joinLoop } from '@/choreography/one-loop';
import { fittedScale } from '../looking/framing';
import { perspectiveFor } from '../looking/named-views';
import { type Turn, makeTurning } from '../looking/turning';
import type { CanvasSize, View } from '../looking/view';
import type { DrawnSpine } from './drawn-spine';
import type { Stage } from './stage';

/** The attribute the canvas wears while a hand may turn it: `free`, `on-a-bone` or `turning`. The stylesheet picks the cursor from it. */
const HAND = 'data-hand';

export interface HandTurningOptions {
  /** The spine was turned or zoomed by hand. */
  onTurned?(): void;
  /** The level under the pointer changed; `null` when it is over no bone. */
  onPoint?(pointed: { spine: DrawnSpine; level: Level } | null): void;
  /** A bone was pressed without dragging; `null` for a press on nothing. */
  onChoose?(chosen: { spine: DrawnSpine; level: Level } | null): void;
}

export interface HandTurning {
  /** Stop any glide now. */
  rest(): void;
  /** Stop listening. */
  detach(): void;
}

export function attachHandTurning(stage: Stage, canvas: HTMLCanvasElement, options: HandTurningOptions = {}): HandTurning {
  const turning = makeTurning({ azimuth: 0, elevation: 0, zoom: 1 });
  let gliding: (() => void) | null = null;
  let pointed: { spine: DrawnSpine; level: Level } | null = null;
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

  /** The view the hand's turn makes, for a canvas of any size. */
  const viewFor = (turn: Turn, from: View, grasped: CanvasSize) => (size: CanvasSize): View => {
    const scale = fittedScale(stage.bounds(), size) * turn.zoom;
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
    const [x, y] = place(event);
    const now = stage.pick(x, y);
    if (now?.spine === pointed?.spine && now?.level === pointed?.level) return;
    pointed = now;
    canvas.setAttribute(HAND, now === null ? 'free' : 'on-a-bone');
    options.onPoint?.(now);
  };

  const onDown = (event: PointerEvent) => {
    if (event.button !== 0 && event.pointerType === 'mouse') return;
    canvas.setPointerCapture(event.pointerId);
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
      if (event.pointerType !== 'touch') point(event);
      return;
    }
    const [x, y] = place(event);
    if (turning.move(event.pointerId, x, y, event.timeStamp)) show();
  };

  const onUp = (event: PointerEvent) => {
    if (!turning.held()) return;
    const { tapped } = turning.release(event.pointerId, event.timeStamp);
    if (turning.held()) return;
    canvas.setAttribute(HAND, pointed === null ? 'free' : 'on-a-bone');
    if (tapped) {
      const [x, y] = place(event);
      options.onChoose?.(stage.pick(x, y));
      return;
    }
    gliding = joinLoop((dt) => {
      // A view set from outside ends the glide: the spine is no longer where the hand threw it.
      if (stage.viewsSet() === mine && turning.glide(dt)) show();
      else {
        gliding?.();
        gliding = null;
      }
    });
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
    if (pointed === null) return;
    pointed = null;
    options.onPoint?.(null);
  };

  canvas.setAttribute(HAND, 'free');
  canvas.addEventListener('pointerdown', onDown);
  canvas.addEventListener('pointermove', onMove);
  canvas.addEventListener('pointerup', onUp);
  canvas.addEventListener('pointercancel', onUp);
  canvas.addEventListener('pointerleave', onLeave);
  canvas.addEventListener('wheel', onWheel, { passive: false });

  return {
    rest,
    detach() {
      rest();
      canvas.removeEventListener('pointerdown', onDown);
      canvas.removeEventListener('pointermove', onMove);
      canvas.removeEventListener('pointerup', onUp);
      canvas.removeEventListener('pointercancel', onUp);
      canvas.removeEventListener('pointerleave', onLeave);
      canvas.removeEventListener('wheel', onWheel);
      canvas.removeAttribute(HAND);
    },
  };
}
