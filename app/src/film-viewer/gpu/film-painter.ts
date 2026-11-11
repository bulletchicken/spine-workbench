// Everything the GPU holds for one film on screen, kept in step with what
// the page asks for: the context, the program, the texture. It keeps the
// film's pixels, look and placement itself, so that after a lost context
// comes back it can make everything again and draw the same picture.

import type { FilmPlacement, FilmSize } from '../film-space';
import { drawFilm, makeFilmProgram, PLAIN_LOOK, type FilmLook, type FilmProgram } from './film-shader';
import { canFilterHalfFloat, uploadFilm, type FilmTexture } from './film-texture';
import { openContext, type FilmSurface } from './gl-context';

/** Whether the film is on screen: drawn, waiting for a lost context to come back, or impossible here. */
export type DrawState = 'drawn' | 'lost' | 'unavailable';

/** How texels are blended: by the GPU's own filtering, or by the shader from exact reads where half floats cannot be filtered. */
export type Blending = 'filtered' | 'by-hand';

export interface FilmPainter {
  /** A film's half-float pixels and its size, or `null` to show nothing. */
  setFilm(pixels: Uint16Array | null, size: FilmSize): void;
  setLook(look: FilmLook): void;
  /** The canvas's size in CSS pixels and the device pixels per CSS pixel. */
  resize(width: number, height: number, devicePixelRatio: number): void;
  /** Draw the film at this placement now. */
  draw(placement: FilmPlacement): void;
  readonly state: DrawState;
  /** How this GPU blends the film's texels; `null` while there is no context to ask. */
  readonly blending: Blending | null;
  stop(): void;
}

interface Made {
  program: FilmProgram;
  texture: FilmTexture | null;
}

export function paintFilm(surface: FilmSurface, corner: number, told: (state: DrawState) => void = () => undefined): FilmPainter {
  let pixels: Uint16Array | null = null;
  let size: FilmSize = { width: 1, height: 1 };
  let look: FilmLook = PLAIN_LOOK;
  let placement: FilmPlacement | null = null;
  let css = { width: 0, height: 0 };
  let made: Made | null = null;
  let state: DrawState = 'drawn';

  const become = (next: DrawState) => {
    if (next === state) return;
    state = next;
    told(next);
  };

  const context = openContext(surface, {
    lost() {
      made = null;
      become('lost');
    },
    restored() {
      make();
      paint();
    },
  });
  const gl = context.gl;

  function make() {
    if (gl === null || gl.isContextLost()) return;
    const filterable = canFilterHalfFloat(gl);
    made = { program: makeFilmProgram(gl, !filterable), texture: null };
    upload();
    become('drawn');
  }

  function upload() {
    if (gl === null || made === null) return;
    if (made.texture !== null) gl.deleteTexture(made.texture.texture);
    made.texture = pixels === null ? null : uploadFilm(gl, pixels, size, !made.program.byHand);
  }

  function paint() {
    if (gl === null || made === null) return;
    gl.viewport(0, 0, surface.width, surface.height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    const texture = made.texture;
    if (texture === null || placement === null || css.width <= 0 || css.height <= 0) return;
    drawFilm(gl, made.program, texture.texture, {
      placement,
      look,
      canvas: { deviceWidth: surface.width, deviceHeight: surface.height, cssWidth: css.width, cssHeight: css.height },
      corner,
      texels: texture.texels,
      step: texture.step,
    });
  }

  // A context can be lost before the painter sees it (its canvas gave it
  // back earlier); then nothing is drawn until the browser restores it.
  if (gl === null) become('unavailable');
  else if (gl.isContextLost()) become('lost');
  else make();

  return {
    setFilm(nextPixels, nextSize) {
      if (nextPixels === pixels && nextSize.width === size.width && nextSize.height === size.height) return;
      pixels = nextPixels;
      size = nextSize;
      upload();
      paint();
    },
    setLook(next) {
      look = next;
      paint();
    },
    resize(width, height, devicePixelRatio) {
      css = { width, height };
      // Setting a canvas's size clears it, so only when it changes.
      const deviceWidth = Math.max(1, Math.round(width * devicePixelRatio));
      const deviceHeight = Math.max(1, Math.round(height * devicePixelRatio));
      if (surface.width !== deviceWidth) surface.width = deviceWidth;
      if (surface.height !== deviceHeight) surface.height = deviceHeight;
      paint();
    },
    draw(next) {
      placement = next;
      paint();
    },
    get state() {
      return state;
    },
    get blending() {
      if (made === null) return null;
      return made.program.byHand ? 'by-hand' : 'filtered';
    },
    stop() {
      if (gl !== null && made !== null && !gl.isContextLost()) {
        if (made.texture !== null) gl.deleteTexture(made.texture.texture);
        gl.deleteProgram(made.program.program);
      }
      made = null;
      context.stop();
    },
  };
}
