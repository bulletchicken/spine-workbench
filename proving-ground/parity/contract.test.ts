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

import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';
import { beforeAll, describe, expect, it } from 'vitest';
import {
  check_case,
  engine_info,
  initSync,
  last_panic,
  pack_case,
  read_case_json,
  unpack_case,
  write_case_json,
} from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import { caseAt, findingsAt, STORY_STAGES } from '../../app/src/sample-case/stages';
import { CASE_FORMAT, CASE_VERSION, SHAPES_FINGERPRINT, type EngineError, type OpenedCase } from '../../app/src/shapes';

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
// Native archive tests read this same three-column manifest.
const archiveVerdicts = readFileSync(`${CASE_FILES}archive-verdicts.tsv`, 'utf8')
  .trimEnd()
  .split('\n')
  .map((line) => {
    const [file, code, detail] = line.split('\t');
    if (file === undefined || code === undefined) throw new Error('archive verdict needs a file and a code');
    return { file, code, detail: detail === '' || detail === undefined ? null : detail };
  });

/** Decode PNG scanlines independently, including all five PNG filter types. */
function pngPixels(png: Uint8Array): { width: number; height: number; bits: number; pixels: Buffer } {
  const bytes = Buffer.from(png);
  expect(bytes.subarray(0, 8)).toEqual(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
  const width = bytes.readUInt32BE(16);
  const height = bytes.readUInt32BE(20);
  const depth = bytes.readUInt8(24);
  expect([8, 16]).toContain(depth);
  expect(bytes.readUInt8(25)).toBe(0);
  let bits = depth;
  const compressed: Buffer[] = [];
  for (let at = 8; at < bytes.length;) {
    const length = bytes.readUInt32BE(at);
    const type = bytes.toString('ascii', at + 4, at + 8);
    const data = bytes.subarray(at + 8, at + 8 + length);
    if (type === 'sBIT') bits = data.readUInt8();
    if (type === 'IDAT') compressed.push(data);
    at += 12 + length;
  }
  const filtered = inflateSync(Buffer.concat(compressed));
  const pixelBytes = depth / 8;
  const stride = width * pixelBytes;
  expect(filtered.length).toBe((stride + 1) * height);
  const pixels = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y += 1) {
    const filter = filtered.readUInt8(y * (stride + 1));
    expect(filter).toBeLessThanOrEqual(4);
    for (let x = 0; x < stride; x += 1) {
      const i = y * stride + x;
      const left = x >= pixelBytes ? pixels.readUInt8(i - pixelBytes) : 0;
      const above = y > 0 ? pixels.readUInt8(i - stride) : 0;
      const upperLeft = y > 0 && x >= pixelBytes ? pixels.readUInt8(i - stride - pixelBytes) : 0;
      const prediction = left + above - upperLeft;
      const distances = [Math.abs(prediction - left), Math.abs(prediction - above), Math.abs(prediction - upperLeft)];
      const nearest = Math.min(...distances);
      const paeth = nearest === distances[0] ? left : nearest === distances[1] ? above : upperLeft;
      const predictor = filter === 0 ? 0 : filter === 1 ? left : filter === 2 ? above : filter === 3 ? Math.floor((left + above) / 2) : paeth;
      pixels[i] = (filtered.readUInt8(y * (stride + 1) + x + 1) + predictor) & 255;
    }
  }
  // PNG permits either zeroes or replicated high bits below sBIT. Only
  // the significant samples are the lossless film values.
  for (let i = 0; i < pixels.length; i += pixelBytes) {
    if (depth === 8) pixels.writeUInt8(pixels.readUInt8(i) >> (depth - bits), i);
    else pixels.writeUInt16BE(pixels.readUInt16BE(i) >> (depth - bits), i);
  }
  return { width, height, bits, pixels };
}

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
      // Side weights are optional and kept out of the repository; both builds are valid.
      trainedSideFinder: expect.any(Boolean) as unknown,
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
    const expected = {
      code: 'case-invalid-value',
      detail: 'marks[0].vertebrae[0].marks.points.supL.at',
    };
    expect(thrownBy(() => write_case_json(offFilm))).toMatchObject(expected);
    expect(thrownBy(() => pack_case({ case: offFilm, films: [] }))).toMatchObject(expected);
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

describe('golden archives cross the real engine', () => {
  it('gives every archive exactly one shared native and wasm verdict', () => {
    const onDisk = ['valid', 'invalid'].flatMap((folder) =>
      readdirSync(`${CASE_FILES}archives/${folder}`).map((file) => `archives/${folder}/${file}`),
    );
    expect(archiveVerdicts.map(({ file }) => file).sort()).toEqual(onDisk.sort());
    expect(new Set(archiveVerdicts.map(({ file }) => file)).size).toBe(archiveVerdicts.length);
  });

  for (const { file, code, detail } of archiveVerdicts) {
    if (code === 'accept') {
      it(`${file} keeps its record and every stored pixel`, () => {
        const opened = unpack_case(readFileSync(`${CASE_FILES}${file}`)) as OpenedCase;
        const jsonFile = file.replace('archives/', '').replace('.spinecase', '.json');
        const text = caseText(jsonFile);
        expect(opened.case).toStrictEqual(JSON.parse(text));
        expect(write_case_json(opened.case)).toBe(text);
        const packed = pack_case(opened);
        const reopened = unpack_case(packed) as OpenedCase;
        expect(reopened.case).toStrictEqual(opened.case);
        expect(reopened.films.map(({ kind }) => kind)).toStrictEqual(opened.films.map(({ kind }) => kind));
        for (const film of reopened.films) {
          const original = opened.films.find(({ kind }) => kind === film.kind);
          if (original === undefined) throw new Error('a packed film was not in the case');
          expect(film.png).toBeInstanceOf(Uint8Array);
          expect(pngPixels(film.png)).toStrictEqual(pngPixels(original.png));
        }
        // Once the writer has put the independent PNG into canonical form,
        // another save is identical, including ZIP headers and PNG bytes.
        expect(pack_case(reopened)).toStrictEqual(packed);
        expect(unpack_case(pack_case(reopened))).toStrictEqual(reopened);
      }, 30_000);
    } else {
      it(`${file} is refused with ${code} at ${String(detail)}`, () => {
        expect(thrownBy(() => unpack_case(readFileSync(`${CASE_FILES}${file}`)))).toStrictEqual({
          code,
          message: expect.any(String) as unknown,
          detail,
        });
      });
    }
  }
});

describe('every sample case stage crosses the real reader unchanged', () => {
  it.each(STORY_STAGES)('%s preserves the complete case', (stage) => {
    const expected = caseAt(stage).case;
    expect(read_case_json(write_case_json(expected))).toStrictEqual(expected);
  });

  it.each(STORY_STAGES)('%s reports the findings recorded by the story', (stage) => {
    expect(check_case(caseAt(stage).case)).toStrictEqual(findingsAt(stage));
  });
});
