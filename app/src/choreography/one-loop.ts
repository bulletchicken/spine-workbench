// The app's single animation loop. Everything that moves by script (the
// pass, a camera glide, an outline easing to new corners) joins this one
// loop, so there is one frame callback however many things are moving and
// none at all when nothing is.

/**
 * What a joiner does on each frame.
 *
 * `dt` is the milliseconds since the frame before, and 0 on the first
 * frame a joiner sees, so a motion starts from where it stands. `now` is
 * the frame's own time, on the clock `performance.now()` reads.
 */
export type FrameStep = (dt: number, now: number) => void;

/** The browser's frames, or a stand-in for them in a test. */
export interface Frames {
  request(callback: (now: number) => void): number;
  cancel(handle: number): void;
}

export interface Loop {
  /**
   * Joins the loop: `step` runs on every frame until the returned function
   * is called. The first joiner starts the frames and the last to leave
   * stops them.
   */
  join(step: FrameStep): () => void;
}

/**
 * The longest a single frame may count for. When the tab is hidden the
 * browser stops sending frames; the first frame back would otherwise carry
 * the whole absence and every motion would leap to its end.
 */
export const LONGEST_FRAME_MS = 100;

export function makeLoop(frames: Frames): Loop {
  /** Each joiner, with the time of the last frame it saw, or null before its first. */
  const joiners = new Map<symbol, { step: FrameStep; last: number | null }>();
  let pending: number | null = null;

  const onFrame = (now: number) => {
    // Asked for first, so a step that throws cannot stop the frames.
    pending = frames.request(onFrame);
    for (const [key, joiner] of [...joiners]) {
      // One that left earlier in this same frame is skipped.
      if (!joiners.has(key)) continue;
      const dt = joiner.last === null ? 0 : Math.min(now - joiner.last, LONGEST_FRAME_MS);
      joiner.last = now;
      joiner.step(dt, now);
    }
  };

  return {
    join(step) {
      const key = Symbol('joiner');
      joiners.set(key, { step, last: null });
      pending ??= frames.request(onFrame);
      return () => {
        joiners.delete(key);
        if (joiners.size === 0 && pending !== null) {
          frames.cancel(pending);
          pending = null;
        }
      };
    },
  };
}

const oneLoop = makeLoop({
  request: (callback) => requestAnimationFrame(callback),
  cancel: (handle) => {
    cancelAnimationFrame(handle);
  },
});

/** Joins the app's one loop. The returned function leaves it. */
export function joinLoop(step: FrameStep): () => void {
  return oneLoop.join(step);
}
