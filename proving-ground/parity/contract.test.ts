// The engine the app ships, run in Node: the release .wasm from
// app/src/engine-room/wasm-build, started the way a worker starts it
// (compile the module, then wasm-bindgen's initSync).
//
// It checks what types alone cannot: that the engine reports the same
// shapes fingerprint and case format as the generated TypeScript, and
// that every golden case file crosses from text into a JavaScript object
// and back with nothing changed on the way (no null turned into
// undefined, no field gained or lost, the same bytes written out), while
// every invalid one is refused with the code and JSON path verdicts.json
// records. Rust reads the same files natively in
// engine/shapes/tests/golden_case_files.rs.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import {
  engine_info,
  initSync,
  last_panic,
  read_case_json,
  write_case_json,
} from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import { CASE_FORMAT, CASE_VERSION, SHAPES_FINGERPRINT, type EngineError } from '../../app/src/shapes';

const REPOSITORY = fileURLToPath(new URL('../..', import.meta.url));
const ENGINE = `${REPOSITORY}app/src/engine-room/wasm-build/wasm_bridge_bg.wasm`;
const CASE_FILES = `${REPOSITORY}proving-ground/known-answers/case-files/`;

interface Verdict {
  file: string;
  verdict: 'accept' | 'refuse';
  code: EngineError['code'] | null;
  detail: string | null;
}

const verdicts = JSON.parse(readFileSync(`${CASE_FILES}verdicts.json`, 'utf8')) as Verdict[];
const caseText = (file: string) => readFileSync(`${CASE_FILES}${file}`, 'utf8');

/** What a call threw, which must be a plain EngineError object. */
function thrownBy(call: () => unknown): unknown {
  try {
    call();
  } catch (thrown) {
    return thrown;
  }
  throw new Error('the call did not throw');
}

/** The workspace version in Cargo.toml, which every engine crate shares. */
function engineVersion(): string {
  const manifest = readFileSync(`${REPOSITORY}Cargo.toml`, 'utf8');
  const version = /\[workspace\.package\]\s+version = "([^"]+)"/.exec(manifest)?.[1];
  if (version === undefined) throw new Error('Cargo.toml has no [workspace.package] version');
  return version;
}

beforeAll(async () => {
  const module = await WebAssembly.compile(readFileSync(ENGINE));
  initSync({ module });
});

describe('the engine reports who it is', () => {
  it('names the same shapes, case format and version as the app', () => {
    expect(engine_info()).toStrictEqual({
      engineVersion: engineVersion(),
      shapesFingerprint: SHAPES_FINGERPRINT,
      caseFormat: CASE_FORMAT,
      caseVersion: CASE_VERSION,
    });
  });

  it('has no panic to report before one happens', () => {
    expect(last_panic()).toBeUndefined();
  });
});

describe('golden case files cross both ways unchanged', () => {
  it('has a verdict for at least one valid and one invalid file', () => {
    expect(verdicts.some((verdict) => verdict.verdict === 'accept')).toBe(true);
    expect(verdicts.some((verdict) => verdict.verdict === 'refuse')).toBe(true);
  });

  for (const { file, verdict, code, detail } of verdicts) {
    if (verdict === 'accept') {
      it(`${file} becomes the same object and the same text again`, () => {
        const text = caseText(file);
        const crossed = read_case_json(text) as unknown;
        expect(crossed).toStrictEqual(JSON.parse(text));
        expect(write_case_json(crossed)).toBe(text);
      });
    } else {
      it(`${file} is refused with ${String(code)} at ${String(detail)}`, () => {
        const thrown = thrownBy(() => read_case_json(caseText(file)));
        expect(thrown).toStrictEqual({ code, message: expect.any(String) as unknown, detail });
      });
    }
  }

  it('checks a case object as strictly as case text', () => {
    const offFilm = JSON.parse(caseText('invalid/point-off-film.json')) as unknown;
    expect(thrownBy(() => write_case_json(offFilm))).toMatchObject({
      code: 'case-invalid-value',
      detail: 'marks[0].vertebrae[0].marks.points.supL.at',
    });
  });

  it('refuses an object with a field the shapes do not have as a stale engine', () => {
    const unknown = JSON.parse(caseText('invalid/unknown-field.json')) as unknown;
    expect(thrownBy(() => write_case_json(unknown))).toMatchObject({
      code: 'engine-out-of-sync',
      detail: 'unknown field films[0].fileName',
    });
  });

  it.each(['constructor', 'toString', 'hasOwnProperty', 'valueOf', '__proto__'])(
    'refuses an unknown field named %s, which every object inherits',
    (name) => {
      const withField = (target: object) => {
        Object.defineProperty(target, name, { value: 1, enumerable: true, writable: true, configurable: true });
      };
      const onFilm = JSON.parse(caseText('valid/minimal.json')) as { films: object[] };
      const film = onFilm.films[0];
      if (film === undefined) throw new Error('minimal.json has a film');
      withField(film);
      expect(thrownBy(() => write_case_json(onFilm))).toMatchObject({
        code: 'engine-out-of-sync',
        detail: `unknown field films[0].${name}`,
      });
      const atTop = JSON.parse(caseText('valid/minimal.json')) as object;
      withField(atTop);
      expect(thrownBy(() => write_case_json(atTop))).toMatchObject({
        code: 'engine-out-of-sync',
        detail: `unknown field ${name}`,
      });
    },
  );

  it('refuses an object with a field left out, even one that may be null', () => {
    const minimal = JSON.parse(caseText('valid/minimal.json')) as { films: Record<string, unknown>[] };
    const film = minimal.films[0];
    if (film === undefined) throw new Error('minimal.json has a film');
    delete film.keptTags;
    expect(thrownBy(() => write_case_json(minimal))).toMatchObject({
      code: 'engine-out-of-sync',
      detail: 'missing field films[0].keptTags',
    });
  });
});
