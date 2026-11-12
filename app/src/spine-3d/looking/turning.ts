// Turning the spine by hand: a drag turns it, it glides a little after
// release, and the wheel or a pinch zooms. Pointer only: no key is
// listened for here. This file is the arithmetic; `scene/hand-turning.ts`
// feeds it a canvas's pointer events.

/** What a hand can change about a view. */
export interface Turn {
  azimuth: number;
  elevation: number;
  /** 1 fits the spine to the canvas. */
  zoom: number;
}

/** Degrees turned for each pixel dragged: round the spine, and over it. */
const ROUND_PER_PX = 0.4;
const OVER_PER_PX = 0.3;
/** How far the view may tip: from a little below the horizon to straight above. */
export const LOWEST = -35;
export const HIGHEST = 90;
/** How far it may be zoomed out and in. */
export const ZOOM_OUT = 0.7;
export const ZOOM_IN = 2.2;
/** A wheel's zoom for each unit it reports. */
const WHEEL_ZOOM = 0.0015;
/** After a second of gliding, this share of the speed is left. */
const GLIDE_LEFT_AFTER_A_SECOND = 0.04;
/** Slower than this, in degrees a second, a glide has stopped. */
const STILL = 0.01;
/** A pointer held still this long before release was placed, not thrown: no glide. */
const HELD_STILL_MS = 80;
/** A press that travelled less than this many pixels is a tap. */
const TAP_WITHIN_PX = 5;

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

export interface Turning {
  /** The view as the hand has left it. */
  now(): Turn;
  /** Start again from a view set from outside; any glide stops. */
  set(turn: Turn): void;
  /** A pointer goes down at a canvas place, at a time in milliseconds. */
  press(pointer: number, x: number, y: number, at: number): void;
  /** A pressed pointer moves. True when the view changed. */
  move(pointer: number, x: number, y: number, at: number): boolean;
  /** A pointer lifts. `tapped` when it was the only one down and hardly moved. */
  release(pointer: number, at: number): { tapped: boolean };
  /** The wheel turns by `deltaY`. */
  wheel(deltaY: number): void;
  /** Carry a glide on by `dt` milliseconds. True while it is still moving. */
  glide(dt: number): boolean;
  /** True while a pointer is down. */
  held(): boolean;
}

export function makeTurning(start: Turn): Turning {
  let turn = { ...start };
  let speed = { round: 0, over: 0 };
  const down = new Map<number, { x: number; y: number }>();
  let travelled = 0;
  let last = { x: 0, y: 0, at: 0 };
  let pinch: { apart: number; zoom: number } | null = null;

  const apart = (): number => {
    const [a, b] = [...down.values()];
    return a === undefined || b === undefined ? 0 : Math.hypot(a.x - b.x, a.y - b.y);
  };

  return {
    now: () => ({ ...turn }),
    set(next) {
      turn = { ...next };
      speed = { round: 0, over: 0 };
    },
    press(pointer, x, y, at) {
      down.set(pointer, { x, y });
      if (down.size === 1) {
        travelled = 0;
        speed = { round: 0, over: 0 };
        last = { x, y, at };
      }
      if (down.size === 2) pinch = { apart: apart(), zoom: turn.zoom };
    },
    move(pointer, x, y, at) {
      const was = down.get(pointer);
      if (was === undefined) return false;
      down.set(pointer, { x, y });
      if (down.size === 2 && pinch !== null && pinch.apart > 0) {
        turn.zoom = clamp((pinch.zoom * apart()) / pinch.apart, ZOOM_OUT, ZOOM_IN);
        return true;
      }
      if (down.size !== 1) return false;
      const [dx, dy] = [x - was.x, y - was.y];
      travelled += Math.abs(dx) + Math.abs(dy);
      turn.azimuth -= dx * ROUND_PER_PX;
      turn.elevation = clamp(turn.elevation + dy * OVER_PER_PX, LOWEST, HIGHEST);
      const seconds = Math.max(1, at - last.at) / 1000;
      speed = { round: (-(x - last.x) * ROUND_PER_PX) / seconds, over: ((y - last.y) * OVER_PER_PX) / seconds };
      last = { x, y, at };
      return true;
    },
    release(pointer, at) {
      if (!down.delete(pointer)) return { tapped: false };
      if (down.size < 2) pinch = null;
      if (down.size > 0) return { tapped: false };
      if (at - last.at > HELD_STILL_MS) speed = { round: 0, over: 0 };
      return { tapped: travelled < TAP_WITHIN_PX };
    },
    wheel(deltaY) {
      turn.zoom = clamp(turn.zoom * Math.exp(-deltaY * WHEEL_ZOOM), ZOOM_OUT, ZOOM_IN);
    },
    glide(dt) {
      if (down.size > 0) return false;
      if (Math.abs(speed.round) <= STILL && Math.abs(speed.over) <= STILL) return false;
      const seconds = dt / 1000;
      turn.azimuth += speed.round * seconds;
      turn.elevation = clamp(turn.elevation + speed.over * seconds, LOWEST, HIGHEST);
      const left = Math.pow(GLIDE_LEFT_AFTER_A_SECOND, seconds);
      speed = { round: speed.round * left, over: speed.over * left };
      return true;
    },
    held: () => down.size > 0,
  };
}
