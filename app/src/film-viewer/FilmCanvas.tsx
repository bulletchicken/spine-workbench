// The film's pixels on screen: a canvas the size of the viewport, drawn
// by the GPU at the film's placement. It never resizes, turns or mirrors
// the image; the placement's matrix does all of that in the shader.
// `place` draws at a new placement at once, without a React render, so a
// camera glide costs one draw call a frame.
//
// The canvas carries `data-blending`: `filtered` where the GPU blends
// half-float texels itself, `by-hand` where the shader has to, so a test
// in a real browser can see which way that browser went.

import { useEffect, useImperativeHandle, useLayoutEffect, useRef, useState, type Ref } from 'react';
import type { Viewport } from './camera/camera';
import type { FilmPlacement } from './film-space';
import { paintFilm, type DrawState, type FilmPainter } from './gpu/film-painter';
import type { FilmLook } from './gpu/film-shader';

export interface FilmCanvasHandle {
  place(placement: FilmPlacement): void;
}

interface FilmCanvasProps {
  /** Half-float display pixels, `placement.size` in size; `null` draws nothing. */
  pixels: Uint16Array | null;
  placement: FilmPlacement;
  look: FilmLook;
  viewport: Viewport;
  /** What the film is, for a screen reader: "Front film". */
  label: string;
  /** Hidden while there is nothing it can draw, so a lost context never shows the browser's placeholder. */
  hidden?: boolean;
  onDrawState?: ((state: DrawState) => void) | undefined;
  ref?: Ref<FilmCanvasHandle>;
}

export function FilmCanvas({ pixels, placement, look, viewport, label, hidden = false, onDrawState, ref }: FilmCanvasProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const painter = useRef<FilmPainter | null>(null);
  const told = useRef(onDrawState);
  const ratio = useDevicePixelRatio();

  useLayoutEffect(() => {
    told.current = onDrawState;
  });

  useLayoutEffect(() => {
    const surface = canvas.current;
    if (surface === null) return undefined;
    // The film's corner radius is a design token.
    const corner = Number.parseFloat(getComputedStyle(surface).getPropertyValue('--r-film')) || 0;
    const made = paintFilm(surface, corner, (state) => {
      showBlending(surface, painter.current);
      told.current?.(state);
    });
    painter.current = made;
    showBlending(surface, made);
    return () => {
      made.stop();
      painter.current = null;
    };
  }, []);

  const { width, height } = placement.size;
  useLayoutEffect(() => {
    painter.current?.setFilm(pixels, { width, height });
  }, [pixels, width, height]);

  useLayoutEffect(() => {
    painter.current?.setLook(look);
  }, [look]);

  useLayoutEffect(() => {
    painter.current?.resize(viewport.width, viewport.height, ratio);
  }, [viewport.width, viewport.height, ratio]);

  useLayoutEffect(() => {
    painter.current?.draw(placement);
  }, [placement]);

  useImperativeHandle(ref, () => ({ place: (next) => painter.current?.draw(next) }), []);

  return <canvas ref={canvas} className="film-canvas" role="img" aria-label={label} hidden={hidden} />;
}

function showBlending(surface: HTMLCanvasElement, painter: FilmPainter | null) {
  const blending = painter?.blending ?? null;
  if (blending === null) surface.removeAttribute('data-blending');
  else surface.setAttribute('data-blending', blending);
}

/** Device pixels per CSS pixel, kept up to date when the window moves to a screen with another, or the page is zoomed. */
function useDevicePixelRatio(): number {
  const [ratio, setRatio] = useState(() => window.devicePixelRatio);
  useEffect(() => {
    // A query that matches the ratio now stops matching the moment it changes.
    const query = window.matchMedia(`(resolution: ${String(ratio)}dppx)`);
    const changed = () => {
      setRatio(window.devicePixelRatio);
    };
    query.addEventListener('change', changed);
    return () => {
      query.removeEventListener('change', changed);
    };
  }, [ratio]);
  return ratio;
}
