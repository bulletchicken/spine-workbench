// The WebGL2 context a film is drawn with, and what happens when the
// browser takes it away. A browser may drop a page's GPU contexts at any
// time (too many tabs, a driver reset, the GPU switched). Calling
// preventDefault on the loss says the page will rebuild, and the browser
// then offers the context back; everything made with the old one is gone
// by then and must be made again from what the page kept.
//
// A browser holds only so many contexts at once (sixteen in Chromium)
// and drops the oldest past that, so a context whose canvas has stopped
// is given back at once rather than when the canvas is collected. It is
// given back at the next microtask checkpoint, once the code that stopped
// it has run to the end, and not at all if the same canvas is opened again
// before then: React, in development, stops and starts every effect once
// on mount, on the same canvas.
//
// A context given back stays lost until the page asks for it again, and
// only the extension object fetched before the loss can ask: a lost
// context hands out no extensions. So that object is kept with the
// canvas, and a canvas opened again later (React's Activity hiding and
// showing it) asks for its context back. The browser then sends
// webglcontextrestored, as after any other loss. It allows the ask only
// once the loss event has been dispatched and its default prevented, so
// an ask that would come earlier waits for that event.

/** The canvas, as far as the context needs it. */
export interface FilmSurface extends EventTarget {
  width: number;
  height: number;
  getContext(kind: 'webgl2', options?: WebGLContextAttributes): WebGL2RenderingContext | null;
}

export interface ContextHandlers {
  /** Everything made with the context is gone; draw nothing until restored. */
  lost(): void;
  /** The context is back, empty: make everything again and draw. */
  restored(): void;
}

export interface FilmContext {
  /** `null` when this browser has no WebGL2. */
  gl: WebGL2RenderingContext | null;
  stop(): void;
}

const ATTRIBUTES: WebGLContextAttributes = {
  alpha: true,
  premultipliedAlpha: true,
  antialias: false,
  depth: false,
  stencil: false,
  preserveDrawingBuffer: false,
};

/** Canvases whose context is waiting to be given back, and the way to call that off. */
const givingBack = new WeakMap<FilmSurface, () => void>();

/** A context given back: the extension that can ask for it again, and whether the browser would allow that yet. */
interface GivenBack {
  lose: WEBGL_lose_context;
  lossDispatched: boolean;
  /** Set by a canvas opened before the loss was dispatched: ask once it has been. */
  askWhenDispatched: boolean;
}

/** Canvases whose context was given back. */
const givenBack = new WeakMap<FilmSurface, GivenBack>();

export function openContext(surface: FilmSurface, handlers: ContextHandlers): FilmContext {
  givingBack.get(surface)?.();
  givingBack.delete(surface);
  const gl = surface.getContext('webgl2', ATTRIBUTES);
  const onLost = (event: Event) => {
    event.preventDefault();
    handlers.lost();
  };
  const onRestored = () => {
    handlers.restored();
  };
  surface.addEventListener('webglcontextlost', onLost);
  surface.addEventListener('webglcontextrestored', onRestored);
  // Kept until the canvas gives its context back again, so a canvas
  // stopped while still lost can be asked for by the next opening.
  const back = givenBack.get(surface);
  const waiting = back !== undefined && gl?.isContextLost() === true ? back : null;
  if (waiting?.lossDispatched === true) waiting.lose.restoreContext();
  else if (waiting !== null) waiting.askWhenDispatched = true;
  return {
    gl,
    stop() {
      if (waiting !== null) waiting.askWhenDispatched = false;
      surface.removeEventListener('webglcontextlost', onLost);
      surface.removeEventListener('webglcontextrestored', onRestored);
      if (gl === null) return;
      let calledOff = false;
      givingBack.set(surface, () => {
        calledOff = true;
      });
      queueMicrotask(() => {
        if (calledOff) return;
        givingBack.delete(surface);
        const lose = gl.isContextLost() ? null : gl.getExtension('WEBGL_lose_context');
        if (lose === null) return;
        const back: GivenBack = { lose, lossDispatched: false, askWhenDispatched: false };
        givenBack.set(surface, back);
        surface.addEventListener(
          'webglcontextlost',
          (event) => {
            event.preventDefault();
            back.lossDispatched = true;
            // The browser records the prevented default after this listener returns.
            if (back.askWhenDispatched) {
              setTimeout(() => {
                if (back.askWhenDispatched) lose.restoreContext();
              }, 0);
            }
          },
          { once: true },
        );
        lose.loseContext();
      });
    },
  };
}
