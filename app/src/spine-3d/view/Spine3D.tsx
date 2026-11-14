// The spine in 3D, as a part a screen can place: a canvas, the stage on it,
// one spine drawn from a fit, and the films it was fitted from if they are
// given. It holds no state of the case and fetches nothing: the fit, the
// bone shapes and the films' pictures all come in as props.
//
// A screen that only wants the spine passes `fit` and `bones`. A screen
// that drives a transition takes the handle and sets values on the stage
// and the spine each frame; nothing here animates by itself except the
// camera's move to a named view, which can be told to jump.

import { useEffect, useImperativeHandle, useLayoutEffect, useRef, type Ref } from 'react';
import type { FilmGeometry, Level, SpineFit } from '@/shapes';
import { joinLoop } from '@/choreography/one-loop';
import { SYSTEM_ASKS_FOR_REDUCED } from '@/choreography/reduced-motion';
import { SPINE_3D_LABEL } from '@/words/spine-3d';
import type { Bone } from '../bones/read-bones';
import type { ViewName } from '../looking/named-views';
import type { View } from '../looking/view';
import { VIEW_MOVE_MS, settle, viewBetween } from '../motion/easing';
import { fittedFilmCamera } from '../placing/fitted-film-camera';
import type { DrawnSpine } from '../scene/drawn-spine';
import type { Pointed } from '../scene/pointed';
import type { FilmPixels } from '../scene/film-wall';
import { type HandTurning, attachHandTurning } from '../scene/hand-turning';
import { modelSpine, type SpineModel } from '../scene/spine-model';
import { type FilmName, type Stage, makeStage } from '../scene/stage';
import './Spine3D.css';

/** A film's facing, turns, stored size and picture. Its scale and origin come from the fit. */
export interface Spine3DFilm {
  geometry: Pick<FilmGeometry, 'quarterTurns' | 'imageRight'>;
  size: readonly [number, number];
  pixels: FilmPixels | null;
}

export interface Spine3DHandle {
  /** The stage, to set anything on. `null` until the canvas is up. */
  stage(): Stage | null;
  /** The spine drawn from `fit`. `null` until there is one. */
  spine(): DrawnSpine | null;
  /** An unlit, non-pointable drawing sharing the fitted buffers. Its caller removes it. */
  addFlatSpine(): DrawnSpine | null;
  /** Move the camera to a named view or to any view: over the design's 600 ms, or at once with `animate: false`. */
  lookFrom(view: ViewName | View, options?: { animate?: boolean }): void;
  /** Every bone solid, whole and in its fitted place, at once. */
  settle(): void;
}

export interface Spine3DProps {
  fit: SpineFit;
  /** The bone shapes, from `loadBones()`. */
  bones: readonly Bone[];
  /** The films the spine was fitted from. A case with only a front film gives only that one. */
  films?: { front?: Spine3DFilm | undefined; side?: Spine3DFilm | undefined };
  /** The view it opens in, and moves to when this changes. */
  view?: ViewName;
  /** Whether a hand may turn and zoom it. */
  turnable?: boolean;
  /** Whether the round shadow lies under it. */
  shadow?: boolean;
  /** The levels to tint. Left out, the bone under the pointer is tinted. */
  highlighted?: readonly Level[];
  /** What a screen reader calls the canvas. */
  label?: string;
  /** The level under the pointer changed; `null` when it is over no bone. */
  onPoint?: (level: Level | null) => void;
  /** A bone was pressed; `null` for a press on nothing. */
  onChoose?: (level: Level | null) => void;
  /** The complete bone or layer identity under the pointer. */
  onPointed?: (pointed: Pointed | null) => void;
  /** The complete identity chosen by a tap. */
  onChosen?: (pointed: Pointed | null) => void;
  /** The spine was turned or zoomed by hand. */
  onTurned?: () => void;
  /** The stage and the spine are ready to be driven, or (with `null`) are about to go. */
  onReady?: (parts: { stage: Stage; spine: DrawnSpine } | null) => void;
  ref?: Ref<Spine3DHandle>;
}

const FILMS: readonly FilmName[] = ['front', 'side'];

/** True when the system, or the app round this part, asks for less motion. */
function reducedMotionAt(part: Element): boolean {
  return window.matchMedia(SYSTEM_ASKS_FOR_REDUCED).matches || part.closest('[data-motion="reduced"]') !== null;
}

export function Spine3D({
  fit,
  bones,
  films,
  view = 'three-quarter',
  turnable = true,
  shadow = true,
  highlighted,
  label = SPINE_3D_LABEL,
  onPoint,
  onChoose,
  onPointed,
  onChosen,
  onTurned,
  onReady,
  ref,
}: Spine3DProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const stage = useRef<Stage | null>(null);
  const spine = useRef<DrawnSpine | null>(null);
  const fittedModel = useRef<SpineModel | null>(null);
  const hand = useRef<HandTurning | null>(null);
  const moving = useRef<(() => void) | null>(null);
  const pointed = useRef<{ spine: DrawnSpine; level: Level } | null>(null);
  /** False until the stage has been given its opening view. */
  const opened = useRef(false);

  // The newest callbacks, whatever functions the caller hands in on each render.
  const told = useRef({ onPoint, onChoose, onPointed, onChosen, onTurned, onReady, highlighted });
  useLayoutEffect(() => {
    told.current = { onPoint, onChoose, onPointed, onChosen, onTurned, onReady, highlighted };
  });

  const lookFrom = useRef((to: ViewName | View, options: { animate?: boolean } = {}) => {
    const on = stage.current;
    const surface = canvas.current;
    if (on === null || surface === null) return;
    moving.current?.();
    moving.current = null;
    hand.current?.rest();
    // A named view is kept as a rule, so it still fits after the canvas changes size.
    const arrive = () => {
      on.setView(typeof to === 'string' ? () => on.namedView(to) : to);
    };
    if (options.animate === false || reducedMotionAt(surface)) {
      arrive();
      return;
    }
    const from = on.view();
    let elapsed = 0;
    let mine = on.viewsSet();
    const leave = joinLoop((dt) => {
      // A view set by anyone else ends this move where it stands.
      if (on.viewsSet() !== mine) {
        leave();
        moving.current = null;
        return;
      }
      elapsed += dt;
      if (elapsed >= VIEW_MOVE_MS) {
        leave();
        moving.current = null;
        arrive();
        return;
      }
      on.setView(viewBetween(from, typeof to === 'string' ? on.namedView(to) : to, settle(elapsed / VIEW_MOVE_MS)));
      mine = on.viewsSet();
    });
    moving.current = leave;
  });

  // The stage lives as long as the canvas does.
  useLayoutEffect(() => {
    const surface = canvas.current;
    if (surface === null) return undefined;
    const made = makeStage(surface);
    stage.current = made;
    return () => {
      moving.current?.();
      moving.current = null;
      stage.current = null;
      opened.current = false;
      made.dispose();
    };
  }, []);

  // One spine for the fit, shaped from the bones.
  useLayoutEffect(() => {
    const on = stage.current;
    if (on === null) return undefined;
    const model = modelSpine(fit, bones); fittedModel.current=model;
    const drawn = on.addSpine(model);
    spine.current = drawn;
    told.current.onReady?.({ stage: on, spine: drawn });
    return () => {
      told.current.onReady?.(null);
      spine.current = null; fittedModel.current=null;
      drawn.remove();
      model.dispose();
    };
  }, [fit, bones]);

  useLayoutEffect(() => {
    spine.current?.setSpine({ shadow: shadow ? 1 : 0 });
  }, [shadow, fit, bones]);

  // The films, each from where the fit says it stands. "Flat" for the spine means flat on the front film.
  const { front, side } = films ?? {};
  useLayoutEffect(() => {
    const on = stage.current;
    if (on === null) return undefined;
    const hung = FILMS.flatMap((name) => {
      const film = name === 'front' ? front : side;
      const placement = name === 'front' ? fit.frontFilm : fit.sideFilm;
      if (film === undefined || placement === null) return [];
      const camera = fittedFilmCamera(film.geometry, film.size, placement);
      return camera === null ? [] : [on.hangFilm(name, camera, film.pixels)];
    });
    return () => {
      for (const wall of hung) wall.remove();
    };
  }, [front, side, fit]);

  useLayoutEffect(() => {
    const on = stage.current;
    spine.current?.pressOnto(on?.film('front') ?? on?.film('side') ?? null);
  }, [front, side, fit, bones]);

  // The view it opens in is set at once; a later change of the prop travels.
  useLayoutEffect(() => {
    lookFrom.current(view, { animate: opened.current });
    opened.current = true;
  }, [view]);

  useEffect(() => {
    const on = stage.current;
    const surface = canvas.current;
    if (!turnable || on === null || surface === null) return undefined;
    const attached = attachHandTurning(on, surface, {
      reducedMotion: () => reducedMotionAt(surface),
      onTurned: () => {
        moving.current?.();
        moving.current = null;
        told.current.onTurned?.();
      },
      onPoint: (found) => {
        const level = found?.level ?? null;
        if (told.current.highlighted === undefined) {
          if (pointed.current !== null) pointed.current.spine.set({ tint: 0 }, pointed.current.level);
          if (found !== null && level !== null) found.spine.set({ tint: 1 }, level);
        }
        pointed.current = found !== null && level !== null ? { spine: found.spine, level } : null;
        told.current.onPoint?.(level);
        told.current.onPointed?.(found);
      },
      onChoose: (found) => { told.current.onChoose?.(found?.level ?? null); told.current.onChosen?.(found); },
    });
    hand.current = attached;
    return () => {
      hand.current = null;
      attached.detach();
    };
  }, [turnable]);

  useLayoutEffect(() => {
    if (highlighted === undefined) return;
    spine.current?.set({ tint: 0 });
    if (highlighted.length > 0) spine.current?.set({ tint: 1 }, highlighted);
  }, [highlighted, fit, bones]);

  useImperativeHandle(
    ref,
    () => ({
      stage: () => stage.current,
      spine: () => spine.current,
      addFlatSpine: () => {
        const on=stage.current,model=fittedModel.current,source=spine.current;
        return on===null||model===null||source===null?null:on.addSpine(model,{shapesFrom:source,flat:true});
      },
      lookFrom: (to, options) => {
        lookFrom.current(to, options);
      },
      settle: () => {
        moving.current?.();
        moving.current = null;
        spine.current?.settle();
      },
    }),
    [],
  );

  return (
    <div className="spine-3d">
      <canvas ref={canvas} className="spine-3d-canvas" role="img" aria-label={label} />
    </div>
  );
}
