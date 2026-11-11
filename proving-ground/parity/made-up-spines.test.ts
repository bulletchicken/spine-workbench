// The same seed gives the same made-up spine natively and in the browser
// build: the release .wasm, run in Node as a worker runs it, must give
// exactly the films and answer key the native engine gives.
//
// The native side is engine/command-line/src/commands/make_up.rs, whose
// test makes the same recipe and checks the same fingerprints: FNV-1a 64
// of each film's PNG bytes, and of the answer key walked the same way on
// both sides (every value as a tag and its contents, numbers as their
// little-endian f64 bytes, object keys in order). Both tests hold the
// numbers below; a change to the generator updates both.
//
// The recipe is random at tier t4, so the films go through every step the
// two builds could part on: the curves, the stacking, the painter, the
// detector's Poisson noise, blurs and logarithms, and the PNG encoder.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { initSync, make_up_spine } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import type { MadeUpRecipe, MadeUpSpine } from '../../app/src/shapes';

const REPOSITORY = fileURLToPath(new URL('../..', import.meta.url));
const ENGINE = `${REPOSITORY}app/src/engine-room/wasm-build/wasm_bridge_bg.wasm`;

const RECIPE: MadeUpRecipe = { name: 'random', seed: 7, tier: 't4' };
const FILM_FINGERPRINTS = ['792bc998dadbd857', '4ccfd22296e9cd69', 'a2f25a44abfd4e99', '1d07a6839bde1423'];
const ANSWER_KEY_FINGERPRINT = '912e2bc926a677d1';

/** FNV-1a 64 over bytes, as 16 hex digits, in two 32-bit halves. */
class Fnv64 {
  private high = 0xcbf29ce4;
  private low = 0x84222325;

  add(bytes: Uint8Array): void {
    let { high, low } = this;
    for (const byte of bytes) {
      low = (low ^ byte) >>> 0;
      // Times the prime 2^40 + 0x1b3: the low half times 0x1b3 carries
      // into the high half, and the low half shifted by 40 lands there too.
      const lowProduct = low * 0x1b3;
      const carry = Math.floor(lowProduct / 0x100000000);
      high = (Math.imul(high, 0x1b3) + carry + (low << 8)) >>> 0;
      low = lowProduct >>> 0;
    }
    this.high = high;
    this.low = low;
  }

  hex(): string {
    return this.high.toString(16).padStart(8, '0') + this.low.toString(16).padStart(8, '0');
  }
}

/** Feeds a value into the fingerprint the way the native test walks it. */
function walk(value: unknown, into: Fnv64): void {
  const tag = (letter: string) => {
    into.add(new Uint8Array([letter.charCodeAt(0)]));
  };
  const text = (s: string) => {
    const bytes = new TextEncoder().encode(s);
    tag('s');
    into.add(new Uint8Array(new Uint32Array([bytes.length]).buffer));
    into.add(bytes);
  };
  if (value === null) {
    tag('n');
  } else if (typeof value === 'boolean') {
    tag(value ? 't' : 'f');
  } else if (typeof value === 'number') {
    tag('d');
    into.add(new Uint8Array(new Float64Array([value]).buffer));
  } else if (typeof value === 'string') {
    text(value);
  } else if (Array.isArray(value)) {
    tag('[');
    for (const item of value) walk(item, into);
    tag(']');
  } else if (typeof value === 'object') {
    tag('{');
    const fields = value as Record<string, unknown>;
    for (const key of Object.keys(fields).sort()) {
      text(key);
      walk(fields[key], into);
    }
    tag('}');
  } else {
    throw new Error(`no fingerprint for a ${typeof value}`);
  }
}

function bytesFingerprint(bytes: Uint8Array): string {
  const into = new Fnv64();
  into.add(bytes);
  return into.hex();
}

function valueFingerprint(value: unknown): string {
  const into = new Fnv64();
  walk(value, into);
  return into.hex();
}

let spine: MadeUpSpine;

// Making up a t4 case takes a few seconds in the release engine.
beforeAll(async () => {
  const module = await WebAssembly.compile(readFileSync(ENGINE));
  initSync({ module });
  spine = make_up_spine(RECIPE) as MadeUpSpine;
}, 120_000);

describe('a made-up spine', () => {
  it('fingerprints as the reference vectors say', () => {
    expect(bytesFingerprint(new TextEncoder().encode(''))).toBe('cbf29ce484222325');
    expect(bytesFingerprint(new TextEncoder().encode('a'))).toBe('af63dc4c8601ec8c');
  });

  it('crosses as four PNG films and an answer key for the recipe', () => {
    expect(spine.films.map((film) => film.kind)).toStrictEqual(['front', 'side', 'bend-left', 'bend-right']);
    for (const film of spine.films) {
      expect(film.png).toBeInstanceOf(Uint8Array);
      expect([...film.png.subarray(1, 4)]).toStrictEqual([0x50, 0x4e, 0x47]);
    }
    expect(spine.answerKey.recipe).toStrictEqual(RECIPE);
  });

  it('paints the same films as the native engine', () => {
    expect(spine.films.map((film) => bytesFingerprint(film.png))).toStrictEqual(FILM_FINGERPRINTS);
  });

  it('has the same answer key as the native engine', () => {
    expect(valueFingerprint(spine.answerKey)).toBe(ANSWER_KEY_FINGERPRINT);
  });
});
