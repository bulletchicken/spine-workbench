// Every case the app's case reducer can reach, checked by the engine's
// own strict reader: the release engine run in Node, as contract.test.ts
// runs it, fed the case after every step of random histories of four
// films (app/src/case-state/random-histories.ts, generated under
// fast-check). The reducer mirrors Rust rules in TypeScript (C7, the
// facing swap, ids, naming); this is what keeps the two from drifting
// apart.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { eachRandomHistory } from '../../app/src/case-state/random-histories';
import { initSync, write_case_json } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';

const ENGINE = fileURLToPath(new URL('../../app/src/engine-room/wasm-build/wasm_bridge_bg.wasm', import.meta.url));

beforeAll(async () => {
  initSync({ module: await WebAssembly.compile(readFileSync(ENGINE)) });
});

describe('the case reducer and the strict reader', () => {
  it('agree on every case a random history reaches', () => {
    let checked = 0;
    eachRandomHistory(300, () => ({ after }) => {
      checked += 1;
      expect(() => write_case_json(after.case)).not.toThrow();
    });
    expect(checked).toBeGreaterThan(1000);
  }, 60_000);
});
