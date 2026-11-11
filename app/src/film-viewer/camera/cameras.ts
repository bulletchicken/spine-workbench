// The four cameras, one per film kind: where each film is on screen now,
// and where it is gliding to. Anything that needs a film's zoom right now
// (a tool judging 12 screen pixels, a scale bar) reads `placementOf`.
//
// Changes made by the pointer land at once. A glide (focus-pull) is a
// target and a start; `step(dt, duration, curve)` moves every gliding
// camera toward its target and lands it exactly when its time is up. A
// glide started while another is under way starts from wherever the
// camera is, so nothing jumps. There is no clock here: whoever owns the
// one animation loop calls `step` with the time since the last frame,
// the token duration and the token curve. Under reduced motion it passes
// a duration of 0, which lands at once; the duration token itself is not
// 0 there, so the caller decides this, not the token. Listeners hear of a
// glide the moment it starts (moving, still at its start), so whoever
// owns the loop can join it then, whoever started the glide.
//
// Front and the film beside it are linked: a change to either moves the
// other through `followPartner`, at once or by gliding, as the change did.
// A film newly linked, or newly hung while linked, takes up the level the
// film already there shows (or is gliding to).

import type { FilmKind, FilmMarks, Px } from '@/shapes';
import type { FilmPlacement, ScreenPoint } from '../film-space';
import { fit, isFit, lookPoint, panBy, travelTo, zoomAt, zoomOverFit, type CameraFrame } from './camera';
import { followPartner } from './follow-partner';

/** An easing curve: progress 0 to 1 in, eased progress out, 0 at 0 and 1 at 1. */
export type Curve = (t: number) => number;

/** Told which film's placement changed, and whether it is still on its way. */
export type CameraListener = (film: FilmKind, moving: boolean) => void;

export interface Cameras {
  /** Hang a film in its viewport, or tell its camera the viewport, margins or view changed. Lands at once. */
  hang(film: FilmKind, frame: CameraFrame): void;
  /** The film has gone. */
  drop(film: FilmKind): void;
  frameOf(film: FilmKind): CameraFrame | null;
  /** Where the film is on screen right now. */
  placementOf(film: FilmKind): FilmPlacement | null;
  /** The zoom as a multiple of the fit: 1 at fit. */
  zoomOverFitOf(film: FilmKind): number | null;
  isGliding(film: FilmKind): boolean;
  fit(film: FilmKind, glide: boolean): void;
  /** The wheel: lands at once. */
  zoomAt(film: FilmKind, factor: number, at: ScreenPoint): void;
  /** Zoom about the middle of the viewport: the + and − keys. */
  zoomBy(film: FilmKind, factor: number, glide: boolean): void;
  /** A drag: lands at once. */
  panBy(film: FilmKind, by: ScreenPoint): void;
  /** Bring a film point to the look spot at `over` times the fit. */
  travelTo(film: FilmKind, point: Px, over: number, glide: boolean): void;
  /** The film that stands beside Front, whose camera follows Front's, and Front's it; `null` for none. */
  link(partner: FilmKind | null): void;
  /** A film's marks, which the link reads to show the same level. */
  setMarks(film: FilmKind, marks: FilmMarks | null): void;
  /** Move every gliding camera on by `dt` milliseconds. True while any is still gliding afterwards, even one a listener started during the step. */
  step(dt: number, duration: number, curve: Curve): boolean;
  subscribe(listener: CameraListener): () => void;
}

interface Glide {
  from: FilmPlacement;
  to: FilmPlacement;
  elapsed: number;
}

interface Camera {
  frame: CameraFrame;
  placement: FilmPlacement;
  glide: Glide | null;
}

export function makeCameras(): Cameras {
  const held = new Map<FilmKind, Camera>();
  const marks = new Map<FilmKind, FilmMarks | null>();
  const listeners = new Set<CameraListener>();
  let partner: FilmKind | null = null;

  const tell = (film: FilmKind, moving: boolean) => {
    for (const listener of listeners) listener(film, moving);
  };

  /** Where a camera is headed: its target while gliding, else where it is. */
  const headedFor = (camera: Camera) => camera.glide?.to ?? camera.placement;

  const land = (film: FilmKind, placement: FilmPlacement) => {
    const camera = held.get(film);
    if (camera === undefined) return;
    camera.placement = placement;
    camera.glide = null;
    tell(film, false);
  };

  const glideTo = (film: FilmKind, placement: FilmPlacement) => {
    const camera = held.get(film);
    if (camera === undefined) return;
    camera.glide = { from: camera.placement, to: placement, elapsed: 0 };
    tell(film, true);
  };

  const move = (film: FilmKind, placement: FilmPlacement, glide: boolean) => {
    if (glide) glideTo(film, placement);
    else land(film, placement);
  };

  const linkedTo = (film: FilmKind): FilmKind | null => {
    if (partner === null) return null;
    if (film === 'front') return partner;
    return film === partner ? 'front' : null;
  };

  /** Move the film linked to `film` so that it shows the level `film` shows at `placement`. */
  const moveFollower = (film: FilmKind, placement: FilmPlacement, glide: boolean) => {
    const other = linkedTo(film);
    const source = held.get(film);
    const follower = other === null ? undefined : held.get(other);
    if (other === null || source === undefined || follower === undefined) return;
    const followed = followPartner(
      { frame: source.frame, placement, marks: marks.get(film) ?? null },
      { frame: follower.frame, marks: marks.get(other) ?? null },
    );
    move(other, followed, glide);
  };

  /** Move a film, and the film linked to it so that it shows the same level. */
  const moveLinked = (film: FilmKind, placement: FilmPlacement, glide: boolean) => {
    move(film, placement, glide);
    moveFollower(film, placement, glide);
  };

  /** Bring the film linked to `film` to the level `film` shows, or is gliding to. */
  const followNow = (film: FilmKind) => {
    const camera = held.get(film);
    if (camera !== undefined) moveFollower(film, headedFor(camera), camera.glide !== null);
  };

  const withCamera = (film: FilmKind, then: (camera: Camera) => void) => {
    const camera = held.get(film);
    if (camera !== undefined) then(camera);
  };

  return {
    hang(film, frame) {
      const camera = held.get(film);
      if (camera !== undefined && camera.frame.size.width === frame.size.width && camera.frame.size.height === frame.size.height) {
        const was = headedFor(camera);
        const placement = isFit(camera.frame, was) ? fit(frame) : travelTo(frame, lookPoint(camera.frame, was), zoomOverFit(camera.frame, was));
        camera.frame = frame;
        land(film, placement);
        return;
      }
      // A film new to this camera: at its fit, then at the level of the film linked to it, if that one is hung.
      held.set(film, { frame, placement: fit(frame), glide: null });
      tell(film, false);
      const other = linkedTo(film);
      if (other !== null) followNow(other);
    },

    drop(film) {
      held.delete(film);
      marks.delete(film);
    },

    frameOf: (film) => held.get(film)?.frame ?? null,

    placementOf: (film) => held.get(film)?.placement ?? null,

    zoomOverFitOf(film) {
      const camera = held.get(film);
      return camera === undefined ? null : zoomOverFit(camera.frame, camera.placement);
    },

    isGliding: (film) => held.get(film)?.glide != null,

    fit(film, glide) {
      withCamera(film, (camera) => {
        moveLinked(film, fit(camera.frame), glide);
      });
    },

    zoomAt(film, factor, at) {
      withCamera(film, (camera) => {
        moveLinked(film, zoomAt(camera.frame, camera.placement, factor, at), false);
      });
    },

    zoomBy(film, factor, glide) {
      withCamera(film, (camera) => {
        const middle: ScreenPoint = [camera.frame.viewport.width / 2, camera.frame.viewport.height / 2];
        moveLinked(film, zoomAt(camera.frame, headedFor(camera), factor, middle), glide);
      });
    },

    panBy(film, by) {
      withCamera(film, (camera) => {
        moveLinked(film, panBy(camera.frame, camera.placement, by), false);
      });
    },

    travelTo(film, point, over, glide) {
      withCamera(film, (camera) => {
        moveLinked(film, travelTo(camera.frame, point, over), glide);
      });
    },

    link(next) {
      const was = partner;
      partner = next === 'front' ? null : next;
      if (partner !== was) followNow('front');
    },

    setMarks(film, filmMarks) {
      marks.set(film, filmMarks);
    },

    step(dt, duration, curve) {
      // Only the glides under way as the step began move on by `dt`: a
      // listener may start or replace one mid-step, and that one starts
      // from where it is, on the next step.
      const under: [FilmKind, Glide][] = [];
      for (const [film, camera] of held) if (camera.glide !== null) under.push([film, camera.glide]);
      for (const [film, glide] of under) {
        const camera = held.get(film);
        if (camera?.glide !== glide) continue;
        glide.elapsed += dt;
        if (duration <= 0 || glide.elapsed >= duration) {
          land(film, glide.to);
          continue;
        }
        camera.placement = between(glide.from, glide.to, curve(glide.elapsed / duration));
        tell(film, true);
      }
      for (const camera of held.values()) if (camera.glide !== null) return true;
      return false;
    },

    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/** A placement `p` of the way from one to another: zoom and pan move together, as one transform. */
function between(from: FilmPlacement, to: FilmPlacement, p: number): FilmPlacement {
  return {
    size: to.size,
    view: to.view,
    zoom: from.zoom + (to.zoom - from.zoom) * p,
    pan: [from.pan[0] + (to.pan[0] - from.pan[0]) * p, from.pan[1] + (to.pan[1] - from.pan[1]) * p],
  };
}

/** The app's cameras. Tools, the bench and the scale bar all read these. */
export const cameras = makeCameras();
