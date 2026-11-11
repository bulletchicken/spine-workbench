// One film on screen: its pixels on the GPU, the layers drawn over it,
// the HUDs over those, and the pointer turned into film pixels for this
// film's running tool or into camera intents.
//
// Where the film sits comes in two ways. The `placement` prop is applied
// whenever its value changes. `place(placement, moving)` on the handle
// moves the film at once without rendering React, for a glide stepped
// on the animation loop: the canvas, the layers' groups and the film's
// glow follow every call, and the layers and HUDs are handed the new
// space once the camera lands (`moving` false), so a glide does not
// re-render every layer every frame.
//
// So a size worked out with `space.screenToFilmLength` grows and shrinks
// with the film during a glide, and settles when the camera lands. A
// part that must keep its size on screen through a glide (a label, a pin
// dot) sizes itself in CSS from `--film-zoom`, the screen pixels per film
// pixel, which the viewer writes on itself at every `place`:
// `font-size: calc(11.5px / var(--film-zoom))`, or a group scaled by
// `calc(1 / var(--film-zoom))` around a film point.
//
// Presses: the right button is window and level, whatever the tool. The
// primary button is shared between the tool and the camera by the rule
// in camera/gestures.ts. A press on a button drawn over the film (a hint
// ring, a choice badge, a HUD's control) is that button's alone.

import {
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type Ref,
} from 'react';
import type { ActiveTool, ToolPointer } from '@/toolbox/tool-contract';
import type { Viewport } from './camera/camera';
import { pressMoved, toolGrabs, wheelZoom, type CameraIntent, type CameraPress } from './camera/gestures';
import { FilmCanvas, type FilmCanvasHandle } from './FilmCanvas';
import { filmSpace, uprightSize, type FilmHud, type FilmLayer, type FilmPlacement, type ScreenPoint } from './film-space';
import type { DrawState } from './gpu/film-painter';
import { PLAIN_LOOK, type FilmLook } from './gpu/film-shader';
import { LayerStack, matrixAttribute } from './LayerStack';
import { watchTool } from './tool-changes';
import { lookAfterDrag } from './window-level-drag';
import './FilmViewer.css';

export interface FilmViewerHandle {
  /** Move the film now. `moving` while a glide is under way; the last call of a glide says false. */
  place(placement: FilmPlacement, moving?: boolean): void;
}

export interface FilmViewerProps {
  /** The film's half-float display pixels, `placement.size` in size; `null` until they arrive. */
  pixels: Uint16Array | null;
  placement: FilmPlacement;
  layers?: readonly FilmLayer[];
  huds?: readonly FilmHud[];
  /** This film's running tool, or none. */
  tool?: ActiveTool | null;
  /** Window, level and the brightness flip to start from; a new value replaces any right-drag. */
  look?: FilmLook;
  onLookChange?: (look: FilmLook) => void;
  /** The wheel, a drag and a double-click, for whoever holds the cameras. */
  onCamera?: (intent: CameraIntent) => void;
  /** The viewer's size in CSS pixels, when it is first known and whenever it changes. */
  onViewport?: (viewport: Viewport) => void;
  onDrawState?: (state: DrawState) => void;
  /** What the film is, for a screen reader: "Front film". */
  label: string;
  ref?: Ref<FilmViewerHandle>;
}

/** The custom property that holds the film's zoom right now, glides included. */
const ZOOM_PROPERTY = '--film-zoom';

/** How far the film's glow reaches past the viewport before it is cut off, in CSS pixels. */
const GLOW_ROOM = 160;

type Press = { kind: 'none' } | { kind: 'tool' } | { kind: 'look'; from: ScreenPoint; look: FilmLook } | (CameraPress & { down: ToolPointer });

export function FilmViewer({
  pixels,
  placement,
  layers = [],
  huds = [],
  tool = null,
  look: givenLook = PLAIN_LOOK,
  onLookChange,
  onCamera,
  onViewport,
  onDrawState,
  label,
  ref,
}: FilmViewerProps) {
  const root = useRef<HTMLDivElement>(null);
  const canvas = useRef<FilmCanvasHandle>(null);
  const glow = useRef<HTMLDivElement>(null);
  const latest = useRef(placement);
  const press = useRef<Press>({ kind: 'none' });

  // The placement React draws with: the prop when its value changes, or a landed `place`.
  const [given, setGiven] = useState(placement);
  const [resting, setResting] = useState(placement);
  if (!samePlacement(given, placement)) {
    setGiven(placement);
    setResting(placement);
  }

  const [lookFrom, setLookFrom] = useState(givenLook);
  const [look, setLook] = useState(givenLook);
  if (!sameLook(lookFrom, givenLook)) {
    setLookFrom(givenLook);
    setLook(givenLook);
  }

  const [drawState, setDrawState] = useState<DrawState>('drawn');
  const [viewport, setViewport] = useState<Viewport>({ width: 0, height: 0 });
  const viewportNow = useRef(viewport);
  const [, redraw] = useReducer((n: number) => n + 1, 0);

  useLayoutEffect(() => {
    latest.current = resting;
  }, [resting]);

  useLayoutEffect(() => {
    const element = root.current;
    if (element === null) return undefined;
    const measure = () => {
      const box = element.getBoundingClientRect();
      viewportNow.current = { width: box.width, height: box.height };
      setViewport((was) => (was.width === box.width && was.height === box.height ? was : { width: box.width, height: box.height }));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, []);

  // Told once per size, whatever function the caller hands in on each render.
  const tellViewport = useRef(onViewport);
  useLayoutEffect(() => {
    tellViewport.current = onViewport;
  });
  useEffect(() => {
    if (viewport.width > 0 && viewport.height > 0) tellViewport.current?.(viewport);
  }, [viewport]);

  useEffect(() => (tool === null ? undefined : watchTool(tool, redraw)), [tool]);

  useEffect(() => {
    press.current = { kind: 'none' };
  }, [tool]);

  useImperativeHandle(
    ref,
    () => ({
      place(next, moving = false) {
        latest.current = next;
        canvas.current?.place(next);
        const element = root.current;
        if (element !== null) {
          element.style.setProperty(ZOOM_PROPERTY, String(next.zoom));
          const transform = matrixAttribute(filmSpace(next).matrix);
          for (const group of element.querySelectorAll('.layer-stack__layer')) group.setAttribute('transform', transform);
          placeGlow(glow.current, next, viewportNow.current);
        }
        if (!moving) setResting((was) => (samePlacement(was, next) ? was : next));
      },
    }),
    [],
  );

  // The wheel needs a listener that may cancel scrolling, which React's is not.
  const camera = useRef(onCamera);
  useLayoutEffect(() => {
    camera.current = onCamera;
  });
  useEffect(() => {
    const element = root.current;
    if (element === null) return undefined;
    const onWheel = (event: WheelEvent) => {
      const intend = camera.current;
      if (intend === undefined) return;
      event.preventDefault();
      intend({ kind: 'zoom', factor: wheelZoom(event.deltaY, event.deltaMode, event.ctrlKey), at: screenPoint(element, event) });
    };
    element.addEventListener('wheel', onWheel, { passive: false });
    return () => {
      element.removeEventListener('wheel', onWheel);
    };
  }, []);

  const pointerAt = (event: ReactPointerEvent, screen: ScreenPoint): ToolPointer => {
    const now = latest.current;
    const at = filmSpace(now).toFilm(screen);
    const { width, height } = now.size;
    return {
      at,
      screen,
      onFilm: at[0] >= -0.5 && at[0] <= width - 0.5 && at[1] >= -0.5 && at[1] <= height - 0.5,
      alt: event.altKey,
      shift: event.shiftKey,
      device: event.pointerType === 'pen' ? 'pen' : event.pointerType === 'touch' ? 'touch' : 'mouse',
    };
  };

  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (pressedAButton(event)) return;
    const screen = screenPoint(event.currentTarget, event);
    if (event.button === 2) {
      event.currentTarget.setPointerCapture(event.pointerId);
      press.current = { kind: 'look', from: screen, look };
      return;
    }
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    const down = pointerAt(event, screen);
    if (tool !== null && toolGrabs(tool.view().cursor)) {
      press.current = { kind: 'tool' };
      tool.pointerDown(down);
      return;
    }
    press.current = { kind: 'undecided', from: screen, down };
  };

  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const now = press.current;
    const screen = screenPoint(event.currentTarget, event);
    if (now.kind === 'none' || now.kind === 'tool') {
      tool?.pointerMove(pointerAt(event, screen));
      return;
    }
    if (now.kind === 'look') {
      const next = lookAfterDrag(now.look, now.from, screen, viewport);
      setLook(next);
      onLookChange?.(next);
      return;
    }
    const moved = pressMoved(now, screen);
    press.current = { ...moved.press, down: now.down };
    if (moved.pan !== null) onCamera?.({ kind: 'pan', by: moved.pan });
  };

  const onPointerUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    const now = press.current;
    press.current = { kind: 'none' };
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    if (tool === null) return;
    const up = pointerAt(event, screenPoint(event.currentTarget, event));
    if (now.kind === 'tool') tool.pointerUp(up);
    if (now.kind === 'undecided') {
      tool.pointerDown(now.down);
      tool.pointerUp(up);
    }
  };

  const onPointerCancel = () => {
    if (press.current.kind === 'tool') tool?.escape();
    press.current = { kind: 'none' };
  };

  const onPointerLeave = () => {
    if (press.current.kind === 'none') tool?.pointerLeave();
  };

  const onDoubleClick = (event: ReactMouseEvent) => {
    if (!pressedAButton(event)) onCamera?.({ kind: 'fit' });
  };

  // One space per resting placement, so a render for a hover or a right-drag hands the layers the same one.
  const space = useMemo(() => filmSpace(resting), [resting]);
  const view = tool?.view() ?? null;
  const preview = view?.preview?.(space) ?? null;
  const style: CSSProperties & Record<typeof ZOOM_PROPERTY, string> = { [ZOOM_PROPERTY]: String(resting.zoom) };
  if (view !== null) style.cursor = view.cursor;

  return (
    <div
      ref={root}
      className="film-viewer"
      style={style}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerCancel}
      onPointerLeave={onPointerLeave}
      onDoubleClick={onDoubleClick}
      onContextMenu={(event) => {
        event.preventDefault();
      }}
    >
      <div ref={glow} className="film-viewer__glow" hidden={pixels === null || drawState !== 'drawn'} style={glowBox(resting, viewport)} />
      <FilmCanvas
        ref={canvas}
        pixels={pixels}
        placement={resting}
        look={look}
        viewport={viewport}
        label={label}
        hidden={drawState !== 'drawn'}
        onDrawState={(state) => {
          setDrawState(state);
          onDrawState?.(state);
        }}
      />
      <LayerStack space={space} layers={layers} preview={preview} />
      {huds.length > 0 && (
        <div className="film-viewer__huds">
          {huds.map(({ id, Draw }) => (
            <Draw key={id} space={space} viewport={viewport} />
          ))}
        </div>
      )}
    </div>
  );
}

function screenPoint(element: Element, event: { clientX: number; clientY: number }): ScreenPoint {
  const box = element.getBoundingClientRect();
  return [event.clientX - box.left, event.clientY - box.top];
}

/** A press that landed on a button drawn over the film belongs to that button. */
function pressedAButton(event: { target: EventTarget; currentTarget: EventTarget }): boolean {
  if (!(event.target instanceof Element) || !(event.currentTarget instanceof Element)) return false;
  const button = event.target.closest('button, [role="button"], a[href], input, label');
  return button !== null && event.currentTarget.contains(button) && button !== event.currentTarget;
}

/** The film's outline on screen, cut to the viewport plus room for its glow, so a zoomed film is never a huge box. */
function glowBox(placement: FilmPlacement, viewport: Viewport): { left: number; top: number; width: number; height: number } {
  const upright = uprightSize(placement.size, placement.view.quarterTurns);
  const left = Math.max(placement.pan[0], -GLOW_ROOM);
  const top = Math.max(placement.pan[1], -GLOW_ROOM);
  const right = Math.min(placement.pan[0] + upright.width * placement.zoom, viewport.width + GLOW_ROOM);
  const bottom = Math.min(placement.pan[1] + upright.height * placement.zoom, viewport.height + GLOW_ROOM);
  return { left, top, width: Math.max(0, right - left), height: Math.max(0, bottom - top) };
}

function placeGlow(element: HTMLDivElement | null, placement: FilmPlacement, viewport: Viewport) {
  if (element === null) return;
  const box = glowBox(placement, viewport);
  element.style.left = `${String(box.left)}px`;
  element.style.top = `${String(box.top)}px`;
  element.style.width = `${String(box.width)}px`;
  element.style.height = `${String(box.height)}px`;
}

function samePlacement(a: FilmPlacement, b: FilmPlacement): boolean {
  return (
    a.zoom === b.zoom &&
    a.pan[0] === b.pan[0] &&
    a.pan[1] === b.pan[1] &&
    a.size.width === b.size.width &&
    a.size.height === b.size.height &&
    a.view.quarterTurns === b.view.quarterTurns &&
    a.view.mirrored === b.view.mirrored
  );
}

function sameLook(a: FilmLook, b: FilmLook): boolean {
  return a.centre === b.centre && a.width === b.width && a.flipped === b.flipped;
}
