import { describe, expect, it } from 'vitest';
import type { FilmPlacement } from '../film-space';
import { paintFilm, type DrawState } from './film-painter';
import type { FilmSurface } from './gl-context';

// A stand-in for a browser's WebGL2 context. It records every call, by
// name, with constants given as their own names ('LINEAR', 'R16F'), and
// answers the probe's read with whatever this GPU is meant to do.

interface Call {
  name: string;
  args: unknown[];
}

interface FakeGpu {
  /** What reading halfway between a 0 texel and a 1 texel gives, 0 to 255. */
  halfway: number;
  largestTexture: number;
  lost: boolean;
  calls: Call[];
}

function fakeGl(gpu: FakeGpu): WebGL2RenderingContext {
  let made = 0;
  const answers: Record<string, (...args: unknown[]) => unknown> = {
    isContextLost: () => gpu.lost,
    getParameter: (name) => (name === 'MAX_TEXTURE_SIZE' ? gpu.largestTexture : 0),
    getShaderParameter: () => true,
    getProgramParameter: () => true,
    getUniformLocation: (_program, name) => ({ uniform: name }),
    getExtension: (name) =>
      name === 'WEBGL_lose_context'
        ? {
            loseContext: () => {
              gpu.calls.push({ name: 'loseContext', args: [] });
              gpu.lost = true;
            },
            restoreContext: () => {
              gpu.calls.push({ name: 'restoreContext', args: [] });
            },
          }
        : null,
    readPixels: (...args) => {
      const out = args[6];
      if (out instanceof Uint8Array) out.fill(gpu.halfway);
    },
  };
  return new Proxy(
    {},
    {
      get(_target, key) {
        if (typeof key !== 'string') return undefined;
        if (/^[A-Z0-9_]+$/.test(key)) return key;
        return (...args: unknown[]) => {
          gpu.calls.push({ name: key, args });
          const answer = answers[key];
          if (answer !== undefined) return answer(...args);
          if (key.startsWith('create')) {
            made += 1;
            return { made, kind: key };
          }
          return undefined;
        };
      },
    },
  ) as WebGL2RenderingContext;
}

class FakeCanvas extends EventTarget implements FilmSurface {
  width = 300;
  height = 150;
  constructor(private readonly gl: WebGL2RenderingContext | null) {
    super();
  }
  getContext(): WebGL2RenderingContext | null {
    return this.gl;
  }
}

function setUp(halfway = 128, largestTexture = 16384) {
  const gpu: FakeGpu = { halfway, largestTexture, lost: false, calls: [] };
  const canvas = new FakeCanvas(fakeGl(gpu));
  const states: DrawState[] = [];
  const painter = paintFilm(canvas, 3, (state) => states.push(state));
  return { gpu, canvas, painter, states };
}

const placement: FilmPlacement = { size: { width: 4, height: 3 }, view: { quarterTurns: 0, mirrored: false }, zoom: 10, pan: [5, 5] };
const pixels = new Uint16Array(12).fill(0x3800);

const named = (gpu: FakeGpu, name: string) => gpu.calls.filter((call) => call.name === name);
const fragmentSources = (gpu: FakeGpu) =>
  named(gpu, 'shaderSource')
    .map((call) => String(call.args[1]))
    .filter((source) => source.includes('u_window'));
const filters = (gpu: FakeGpu) =>
  named(gpu, 'texParameteri')
    .filter((call) => call.args[1] === 'TEXTURE_MAG_FILTER')
    .map((call) => call.args[2]);

describe('the film painter', () => {
  it('uploads the film as an R16F half-float texture, filtered, when the GPU blends half floats', () => {
    const { gpu, painter } = setUp(128);
    painter.resize(40, 30, 2);
    painter.setFilm(pixels, placement.size);
    painter.draw(placement);
    const storage = named(gpu, 'texStorage2D');
    expect(storage).toHaveLength(1);
    expect(storage[0]?.args).toEqual(['TEXTURE_2D', 1, 'R16F', 4, 3]);
    const upload = named(gpu, 'texSubImage2D');
    expect(upload[0]?.args.slice(6, 9)).toEqual(['RED', 'HALF_FLOAT', pixels]);
    // The probe's texture, then the film's: both linear.
    expect(filters(gpu)).toEqual(['LINEAR', 'LINEAR']);
    expect(fragmentSources(gpu).every((source) => !source.includes('#define BY_HAND'))).toBe(true);
    expect(named(gpu, 'drawArrays').length).toBeGreaterThanOrEqual(2);
    expect(painter.state).toBe('drawn');
    expect(painter.blending).toBe('filtered');
  });

  it('blends by hand from unfiltered texels on a GPU that cannot filter half floats', () => {
    for (const halfway of [0, 255]) {
      const { gpu, painter } = setUp(halfway);
      painter.resize(40, 30, 1);
      painter.setFilm(pixels, placement.size);
      painter.draw(placement);
      // The probe still asks for linear; the film is then read texel by texel.
      expect(filters(gpu)).toEqual(['LINEAR', 'NEAREST']);
      expect(fragmentSources(gpu)).toHaveLength(1);
      expect(fragmentSources(gpu)[0]).toContain('#define BY_HAND');
      expect(painter.state).toBe('drawn');
      expect(painter.blending).toBe('by-hand');
    }
  });

  it('makes everything again and draws the same picture after the context is lost and restored', () => {
    const { gpu, canvas, painter, states } = setUp(128);
    painter.resize(40, 30, 2);
    painter.setFilm(pixels, placement.size);
    painter.draw(placement);
    const drawsBefore = named(gpu, 'drawArrays').length;

    gpu.lost = true;
    const loss = new Event('webglcontextlost', { cancelable: true });
    canvas.dispatchEvent(loss);
    // preventDefault is what asks the browser to give the context back.
    expect(loss.defaultPrevented).toBe(true);
    expect(painter.state).toBe('lost');
    painter.draw({ ...placement, zoom: 12 });
    expect(named(gpu, 'drawArrays')).toHaveLength(drawsBefore);

    gpu.lost = false;
    gpu.calls.length = 0;
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expect(painter.state).toBe('drawn');
    expect(states).toEqual(['lost', 'drawn']);
    // A new program, the film uploaded again from the pixels kept, and a draw at the last placement.
    expect(named(gpu, 'linkProgram').length).toBeGreaterThanOrEqual(1);
    expect(named(gpu, 'texSubImage2D')[0]?.args[8]).toBe(pixels);
    expect(named(gpu, 'drawArrays').length).toBeGreaterThanOrEqual(2);
    const toFilm = named(gpu, 'uniformMatrix3fv').at(-1)?.args[2] as number[];
    // At zoom 12 a screen pixel is 1/12 of a film pixel.
    expect(toFilm[0]).toBeCloseTo(1 / 12, 12);
  });

  it('says so when the browser has no WebGL2, and does nothing else', () => {
    const states: DrawState[] = [];
    const painter = paintFilm(new FakeCanvas(null), 3, (state) => states.push(state));
    painter.resize(40, 30, 1);
    painter.setFilm(pixels, placement.size);
    painter.draw(placement);
    expect(painter.state).toBe('unavailable');
    expect(states).toEqual(['unavailable']);
    expect(painter.blending).toBeNull();
  });

  it('says the film is lost, not drawn, when its context is already lost as the canvas opens, and draws once it is back', () => {
    const gpu: FakeGpu = { halfway: 128, largestTexture: 16384, lost: true, calls: [] };
    const canvas = new FakeCanvas(fakeGl(gpu));
    const states: DrawState[] = [];
    const painter = paintFilm(canvas, 3, (state) => states.push(state));
    painter.resize(40, 30, 1);
    painter.setFilm(pixels, placement.size);
    painter.draw(placement);
    expect(painter.state).toBe('lost');
    expect(painter.blending).toBeNull();
    expect(named(gpu, 'drawArrays')).toHaveLength(0);
    gpu.lost = false;
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expect(painter.state).toBe('drawn');
    expect(states).toEqual(['lost', 'drawn']);
    expect(named(gpu, 'texSubImage2D')[0]?.args[8]).toBe(pixels);
    expect(named(gpu, 'drawArrays').length).toBeGreaterThanOrEqual(2);
  });

  it('gives the context back once stopped, at the next microtask checkpoint', async () => {
    const { gpu, painter } = setUp();
    painter.stop();
    expect(named(gpu, 'loseContext')).toHaveLength(0);
    await Promise.resolve();
    expect(named(gpu, 'loseContext')).toHaveLength(1);
  });

  it('keeps the context when the same canvas is opened again straight away, as React does in development', async () => {
    const { gpu, canvas, painter } = setUp();
    painter.stop();
    const again = paintFilm(canvas, 3);
    await Promise.resolve();
    expect(named(gpu, 'loseContext')).toHaveLength(0);
    expect(again.state).toBe('drawn');
    again.stop();
    await Promise.resolve();
    expect(named(gpu, 'loseContext')).toHaveLength(1);
  });

  it('asks for the context back when a canvas that gave it back is opened again, and draws once it is back', async () => {
    const { gpu, canvas, painter } = setUp();
    painter.stop();
    await Promise.resolve();
    expect(named(gpu, 'loseContext')).toHaveLength(1);
    const loss = new Event('webglcontextlost', { cancelable: true });
    canvas.dispatchEvent(loss);
    // Without a prevented default the browser would refuse to give it back.
    expect(loss.defaultPrevented).toBe(true);

    const states: DrawState[] = [];
    const again = paintFilm(canvas, 3, (state) => states.push(state));
    expect(again.state).toBe('lost');
    expect(named(gpu, 'restoreContext')).toHaveLength(1);
    again.resize(40, 30, 1);
    again.setFilm(pixels, placement.size);
    again.draw(placement);
    gpu.lost = false;
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expect(again.state).toBe('drawn');
    expect(states).toEqual(['lost', 'drawn']);
    expect(named(gpu, 'texSubImage2D').at(-1)?.args[8]).toBe(pixels);
  });

  it('waits for the loss to be dispatched before asking for the context back, when opened again in between', async () => {
    const { gpu, canvas, painter } = setUp();
    painter.stop();
    await Promise.resolve();
    const again = paintFilm(canvas, 3);
    expect(again.state).toBe('lost');
    expect(named(gpu, 'restoreContext')).toHaveLength(0);
    canvas.dispatchEvent(new Event('webglcontextlost', { cancelable: true }));
    // Not inside the loss event, where the browser has not yet recorded the prevented default.
    expect(named(gpu, 'restoreContext')).toHaveLength(0);
    await new Promise((settle) => setTimeout(settle, 0));
    expect(named(gpu, 'restoreContext')).toHaveLength(1);
  });

  it('asks nothing for a canvas stopped while still lost, and asks for the next one opened', async () => {
    const { gpu, canvas, painter } = setUp();
    painter.stop();
    await Promise.resolve();
    // Opened before the loss is dispatched, so its ask waits; stopped before the ask is made.
    const brief = paintFilm(canvas, 3);
    canvas.dispatchEvent(new Event('webglcontextlost', { cancelable: true }));
    brief.stop();
    await new Promise((settle) => setTimeout(settle, 0));
    expect(named(gpu, 'restoreContext')).toHaveLength(0);
    expect(named(gpu, 'loseContext')).toHaveLength(1);
    const third = paintFilm(canvas, 3);
    expect(third.state).toBe('lost');
    expect(named(gpu, 'restoreContext')).toHaveLength(1);
  });

  it('sizes the canvas in device pixels and leaves it alone when nothing changed', () => {
    const { gpu, canvas, painter } = setUp();
    painter.resize(333.5, 200, 2);
    expect([canvas.width, canvas.height]).toEqual([667, 400]);
    painter.setFilm(pixels, placement.size);
    painter.draw(placement);
    const cssPerDevice = named(gpu, 'uniform2f').filter((call) => (call.args[0] as { uniform: string }).uniform === 'u_cssPerDevice');
    expect(cssPerDevice.at(-1)?.args.slice(1)).toEqual([333.5 / 667, 0.5]);
  });

  it('shows a smaller copy of a film too large for the GPU, and says how much smaller', () => {
    const { gpu, painter } = setUp(128, 2);
    painter.resize(40, 30, 1);
    painter.setFilm(pixels, placement.size);
    painter.draw(placement);
    expect(named(gpu, 'texStorage2D')[0]?.args).toEqual(['TEXTURE_2D', 1, 'R16F', 2, 2]);
    const step = named(gpu, 'uniform1f').filter((call) => (call.args[0] as { uniform: string }).uniform === 'u_step');
    expect(step.at(-1)?.args[1]).toBe(2);
  });
});
