// Native recorded checkpoints and the release browser build share one pass.
// Identical strip bytes isolate the chain's libm arithmetic from the FFT.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { ChainPass, FilmDesk, initSync, make_up_spine } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import { runSteps } from '../../app/src/engine-room/workers/step-loop';
import type {
  AutoPassInput,
  AutoPassResult,
  ChainAnswer,
  FilmMarks,
  FilmPassResult,
  MadeUpRecipe,
  MadeUpSpine,
  PassWarning,
  Step,
  WatchFrame,
} from '../../app/src/shapes';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const FIXTURES = `${ROOT}proving-ground/known-answers/recorded-frames/`;
const ENGINE = `${ROOT}app/src/engine-room/wasm-build/wasm_bridge_bg.wasm`;

interface Recording {
  recipe: MadeUpRecipe;
  input: AutoPassInput;
  result: AutoPassResult;
  frames: WatchFrame[];
  filmResult: FilmPassResult;
  chainAnswer: ChainAnswer;
}

const payloads = new Uint8Array(readFileSync(`${FIXTURES}frame-payloads.bin`));
const recording = restore(JSON.parse(readFileSync(`${FIXTURES}frames.json`, 'utf8')) as unknown) as Recording;
const nativeStrip = new Uint8Array(readFileSync(`${FIXTURES}front.strip`));

/** Restore the fixture's compact storage to fresh byte fields. */
function restore(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(restore);
  if (value === null || typeof value !== 'object') return value;
  const fields = value as Record<string, unknown>;
  if ('byteOffset' in fields) {
    const { byteOffset, byteLength, length, encoding } = fields;
    if (typeof byteOffset !== 'number' || typeof byteLength !== 'number' || typeof length !== 'number') {
      throw new Error('a recorded byte field has no integer bounds');
    }
    if (![byteOffset, byteLength, length].every((n) => Number.isInteger(n) && n >= 0) || byteOffset + byteLength > payloads.length) {
      throw new Error('a recorded byte field lies outside its payload');
    }
    const encoded = payloads.slice(byteOffset, byteOffset + byteLength);
    if (encoding === 'raw' && encoded.length === length) return encoded;
    if (encoding !== 'runs' || encoded.length % 2 !== 0) throw new Error('a recorded byte encoding is invalid');
    const decoded = new Uint8Array(length);
    let at = 0;
    for (let i = 0; i < encoded.length; i += 2) {
      const count = encoded[i] ?? 0;
      const byte = encoded[i + 1] ?? 0;
      if (count === 0 || at + count > length) throw new Error('a recorded byte run has invalid bounds');
      decoded.fill(byte, at, at + count);
      at += count;
    }
    if (at !== length) throw new Error('a recorded byte field ends early');
    return decoded;
  }
  return Object.fromEntries(Object.entries(fields).map(([key, item]) => [key, restore(item)]));
}

function plain(value: unknown): unknown {
  if (value instanceof Uint8Array) return Array.from(value);
  if (Array.isArray(value)) return value.map(plain);
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, plain(item)]));
  }
  return value;
}

/** FFT sums may differ in the last f32 bits between SIMD kernels. The
 * tolerance is 2e-5 relative plus 1e-5 absolute: below a preview byte and
 * far below a film pixel; discrete decisions must still agree. */
function near(actual: unknown, expected: unknown, path = 'value'): void {
  if (typeof actual === 'number' && typeof expected === 'number') {
    if (Object.is(actual, expected) || (Number.isNaN(actual) && Number.isNaN(expected))) return;
    const tolerance = 1e-5 + 2e-5 * Math.abs(expected);
    if (!Number.isFinite(actual) || !Number.isFinite(expected) || Math.abs(actual - expected) > tolerance) {
      throw new Error(`${path}: ${String(actual)} differs from ${String(expected)} (tolerance ${String(tolerance)})`);
    }
    return;
  }
  if (Array.isArray(actual) && Array.isArray(expected)) {
    expect(actual.length, path).toBe(expected.length);
    for (let i = 0; i < actual.length; i++) near(actual[i], expected[i], `${path}[${String(i)}]`);
    return;
  }
  if (actual !== null && expected !== null && typeof actual === 'object' && typeof expected === 'object') {
    const left = actual as Record<string, unknown>;
    const right = expected as Record<string, unknown>;
    expect(Object.keys(left).sort(), path).toStrictEqual(Object.keys(right).sort());
    for (const key of Object.keys(left)) near(left[key], right[key], `${path}.${key}`);
    return;
  }
  expect(actual, path).toStrictEqual(expected);
}

function bytesNear(actual: Uint8Array, expected: Uint8Array, name: string, weakHessian = false): void {
  expect(actual.length, name).toBe(expected.length);
  for (let i = 0; i < actual.length; i++) {
    const left = actual[i] ?? 0;
    const right = expected[i] ?? 0;
    // Quantisation may straddle one byte boundary. In Hessian responses
    // below 1/32 strength, dividing FFT cancellation noise by the 1e-6
    // contrast floor permits 4/255 absolute drift. Strong responses still
    // get only one byte; masks, indices and the final marks stay stricter.
    const tolerance = weakHessian && Math.max(left, right) <= 8 ? 4 : 1;
    if (Math.abs(left - right) > tolerance) {
      throw new Error(`${name}[${String(i)}] differs by more than ${String(tolerance)} preview bytes`);
    }
  }
}

function floats(bytes: Uint8Array): number[] {
  const copy = Uint8Array.from(bytes);
  const view = new DataView(copy.buffer);
  return Array.from({ length: copy.length / 4 }, (_, i) => view.getFloat32(i * 4, true));
}

function frameNear(actual: WatchFrame, expected: WatchFrame): void {
  expect(actual.kind).toBe(expected.kind);
  if (actual.kind === 'evidence' && expected.kind === 'evidence') {
    if (actual.tile.encoding === 'mask' || actual.tile.encoding === 'index' || actual.map === 'image') {
      expect(Array.from(actual.tile.data)).toStrictEqual(Array.from(expected.tile.data));
    } else {
      bytesNear(actual.tile.data, expected.tile.data, `${actual.map}.${String(actual.step)}.tile.data`, actual.map === 'hessian-scale' || actual.map === 'valley');
    }
    near(plain({ ...actual, tile: { ...actual.tile, data: [] } }), plain({ ...expected, tile: { ...expected.tile, data: [] } }));
  } else if (actual.kind === 'strip-rows' && expected.kind === 'strip-rows') {
    bytesNear(actual.image, expected.image, 'strip.image');
    near(plain({ ...actual, image: [] }), plain({ ...expected, image: [] }));
  } else if (actual.kind === 'barcode' && expected.kind === 'barcode') {
    near(floats(actual.values), floats(expected.values), 'barcode.values');
    near(floats(actual.highPass), floats(expected.highPass), 'barcode.highPass');
    near({ ...actual, values: [], highPass: [] }, { ...expected, values: [], highPass: [] });
  } else if (actual.kind === 'sweep' && expected.kind === 'sweep') {
    bytesNear(actual.heat, expected.heat, 'sweep.heat');
    near(plain({ ...actual, heat: [] }), plain({ ...expected, heat: [] }));
  } else {
    near(plain(actual), plain(expected));
  }
}

function finishChain(input: FilmPassResult, units: readonly number[]): { answer: ChainAnswer; frames: WatchFrame[] } {
  const pass = ChainPass.start([input], recording.input.options);
  const frames: WatchFrame[] = [];
  let consumed = false;
  try {
    for (let i = 0; ; i++) {
      const step = pass.advance(units[i % units.length] ?? 1) as Step;
      frames.push(...step.frames);
      if (step.done) break;
    }
    consumed = true;
    return { answer: pass.finish() as ChainAnswer, frames };
  } finally {
    if (!consumed) pass.free();
  }
}

let full: { result: AutoPassResult; frames: WatchFrame[] };

beforeAll(async () => {
  initSync({ module: await WebAssembly.compile(readFileSync(ENGINE)) });
  const spine = make_up_spine(recording.recipe) as MadeUpSpine;
  const png = spine.films.find((film) => film.kind === 'front');
  const film = recording.input.films[0];
  if (png === undefined || film === undefined) throw new Error('the recorded front film is missing');
  const desk = FilmDesk.open(png.png, 'front');
  try {
    const pass = desk.start_film_pass(film);
    const frames: WatchFrame[] = [];
    let consumed = false;
    let filmResult: FilmPassResult;
    try {
      for (let i = 0; ; i++) {
        const step = pass.advance([1, 7, 2, 31][i % 4] ?? 1) as Step;
        frames.push(...step.frames);
        if (step.done) break;
      }
      consumed = true;
      filmResult = pass.finish() as FilmPassResult;
    } finally {
      if (!consumed) pass.free();
    }
    expect(filmResult.strip).toBeInstanceOf(Uint8Array);
    const chain = finishChain(filmResult, [3, 1, 19]);
    frames.push(...chain.frames);
    const filmChain = chain.answer.films[0];
    if (filmChain === undefined) throw new Error('the chain returned no front film');
    expect(filmChain.chain).toBeInstanceOf(Uint8Array);
    const marks = desk.place_points(filmChain, recording.input.firstVertebraId) as FilmMarks;
    const warnings: PassWarning[] = [];
    for (const warning of [...filmResult.warnings, ...filmChain.warnings]) {
      if (!warnings.some((existing) => JSON.stringify(existing) === JSON.stringify(warning))) warnings.push(warning);
    }
    full = { result: { caseVersion: recording.input.caseVersion, marks: [marks], twoFilms: chain.answer.twoFilms, warnings }, frames };
  } finally {
    desk.free();
  }
}, 300_000);

describe('the native and browser auto pass', () => {
  it('agrees exactly on chain arithmetic when the native strip bytes are shared', () => {
    const chain = finishChain({ ...recording.filmResult, strip: nativeStrip }, [11, 1, 4]);
    expect(plain(chain.answer)).toStrictEqual(plain(recording.chainAnswer));
    const nativeFrames = recording.frames.filter((frame) => ['chain', 'naming', 'confidence'].includes(frame.stage));
    expect(plain(chain.frames)).toStrictEqual(plain(nativeFrames));
  }, 120_000);

  it('keeps every fixed checkpoint when work units change between slices', () => {
    expect(full.frames.length).toBe(recording.frames.length);
    full.frames.forEach((frame, index) => {
      const native = recording.frames[index];
      if (native === undefined) throw new Error('a native frame is missing');
      frameNear(frame, native);
    });
  });

  it('returns the native marks within the stated FFT tolerance', () => {
    near(plain(full.result), plain(recording.result));
  });

  it('cancels the browser build between real sweep checkpoints and frees it once', async () => {
    const pass = ChainPass.start([{ ...recording.filmResult, strip: nativeStrip }], recording.input.options);
    let time = 0;
    let stopped = false;
    let sawSweep = false;
    let freed = 0;
    let finished = 0;
    const sent: WatchFrame[] = [];
    const outcome = await runSteps(
      {
        advance: (units) => {
          const step = pass.advance(units) as Step;
          time += 8;
          return step;
        },
        finish: () => {
          finished++;
          return pass.finish() as ChainAnswer;
        },
        free: () => {
          freed++;
          pass.free();
        },
      },
      {
        progress: () => undefined,
        frame: (frame) => {
          expect(stopped).toBe(false);
          sent.push(frame);
          sawSweep ||= frame.kind === 'sweep';
        },
      },
      () => stopped,
      {
        now: () => time,
        pause: () => {
          stopped = sawSweep;
          return Promise.resolve();
        },
      },
    );
    expect(sawSweep).toBe(true);
    expect(outcome).toStrictEqual({ kind: 'cancelled' });
    expect(freed).toBe(1);
    expect(finished).toBe(0);
    expect(sent.some((frame) => frame.kind === 'chain-locked')).toBe(false);
  }, 120_000);
});
