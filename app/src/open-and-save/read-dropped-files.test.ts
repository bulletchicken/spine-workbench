import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { MAX_CASE_BYTES, MAX_FILM_BYTES, readDroppedFiles, type DroppedFile, type ReadLimits } from './read-dropped-files';

const ZIP = [0x50, 0x4b, 0x03, 0x04];
const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const MiB = 1024 * 1024;

function blob(bytes: number[], name = 'anything'): Blob {
  // A name can be set on a File, and must make no difference to the answer.
  return new File([new Uint8Array(bytes)], name);
}

/** A blob that claims a size and shows only its first bytes: for limits, without megabytes of memory. */
function claimed(size: number, head: number[]): Blob {
  return {
    size,
    slice: () => new Blob([new Uint8Array(head)]),
    arrayBuffer: () => Promise.resolve(new ArrayBuffer(size)),
  } as unknown as Blob;
}

/** A blob the system will not give the page: its first bytes cannot be read. */
function unreadable(size: number): Blob {
  return {
    size,
    slice: () => ({ arrayBuffer: () => Promise.reject(new Error('the file was moved')) }),
  } as unknown as Blob;
}

/** The bytes an entry holds, read as the opening step would read them. */
async function bytesOf(entry: DroppedFile | undefined): Promise<number[]> {
  if (entry === undefined || entry.kind === 'refused') throw new Error('a refused or missing file holds no bytes');
  return [...new Uint8Array(await entry.blob.arrayBuffer())];
}

const SMALL: ReadLimits = { filmBytes: 16, caseBytes: 32 };

describe('telling dropped files apart', () => {
  it('reads a zip as a case file and anything else as a film', async () => {
    const out = await readDroppedFiles([blob([...ZIP, 1, 2, 3]), blob([...PNG, 9])]);
    expect(out.map((entry) => entry.kind)).toEqual(['case', 'film']);
    expect(await bytesOf(out[0])).toEqual([...ZIP, 1, 2, 3]);
    expect(await bytesOf(out[1])).toEqual([...PNG, 9]);
  });

  it('judges by the bytes, never by the name', async () => {
    const out = await readDroppedFiles([blob([...ZIP, 0], 'scan.png'), blob([...PNG, 0], 'case.spinecase')]);
    expect(out.map((entry) => entry.kind)).toEqual(['case', 'film']);
  });

  it('treats a file shorter than the zip signature as a film, not a case', async () => {
    const out = await readDroppedFiles([blob([0x50, 0x4b])]);
    expect(out.map((entry) => entry.kind)).toEqual(['film']);
  });

  it('keeps the order the files were given in, refusals included', async () => {
    const out = await readDroppedFiles([blob([...PNG, 1]), blob([...ZIP, 2]), blob([]), blob([...PNG, 3])]);
    expect(out.map((entry) => entry.kind)).toEqual(['film', 'case', 'refused', 'film']);
    expect(await bytesOf(out[0])).toEqual([...PNG, 1]);
    expect(await bytesOf(out[3])).toEqual([...PNG, 3]);
  });

  it('refuses an empty file with the code for a file that ends too soon', async () => {
    const out = await readDroppedFiles([blob([])]);
    expect(out).toEqual([{ kind: 'refused', code: 'film-truncated' }]);
  });

  it('refuses a file the system will not read, and still reads the others beside it', async () => {
    const good = blob([...PNG, 5]);
    const out = await readDroppedFiles([good, unreadable(20), blob([...ZIP, 6])]);
    expect(out.map((entry) => entry.kind)).toEqual(['film', 'refused', 'case']);
    expect(out[1]).toEqual({ kind: 'refused', code: 'film-truncated' });
    expect(await bytesOf(out[0])).toEqual([...PNG, 5]);
  });

  it('keeps each file as a plain blob that carries no name', async () => {
    const out = await readDroppedFiles([blob([...PNG, 1], 'scan.png'), blob([...ZIP, 2], 'case.spinecase')]);
    for (const entry of out) {
      if (entry.kind === 'refused') throw new Error('nothing here should be refused');
      expect(entry.blob).not.toBeInstanceOf(File);
      expect('name' in entry.blob).toBe(false);
    }
    expect(await bytesOf(out[0])).toEqual([...PNG, 1]);
    expect(await bytesOf(out[1])).toEqual([...ZIP, 2]);
  });
});

describe('size limits', () => {
  it('the film limit is the engine intake limit, 128 MiB', () => {
    expect(MAX_FILM_BYTES).toBe(128 * MiB);
  });

  it('the case limit is four films at 40 million pixels, two bytes a pixel, plus 10 percent', () => {
    expect(MAX_CASE_BYTES).toBe(352_000_000);
    expect(MAX_CASE_BYTES).toBeGreaterThan(4 * 40_000_000 * 2);
  });

  it('refuses a film over 128 MiB as too large, and reads the others', async () => {
    const out = await readDroppedFiles([claimed(MAX_FILM_BYTES + 1, PNG), blob([...PNG])]);
    expect(out.map((entry) => entry.kind)).toEqual(['refused', 'film']);
    expect(out[0]).toEqual({ kind: 'refused', code: 'film-too-large' });
  });

  it('a film exactly at the limit is read', async () => {
    const out = await readDroppedFiles([claimed(MAX_FILM_BYTES, PNG)]);
    expect(out.map((entry) => entry.kind)).toEqual(['film']);
  });

  it('refuses a case file over its limit as too large, judged by its signature, not its size alone', async () => {
    const out = await readDroppedFiles([claimed(MAX_CASE_BYTES + 1, ZIP)]);
    expect(out).toEqual([{ kind: 'refused', code: 'case-too-large' }]);
  });

  it('a case file between the film limit and the case limit is read', async () => {
    const out = await readDroppedFiles([claimed(MAX_FILM_BYTES + 1, ZIP)]);
    expect(out.map((entry) => entry.kind)).toEqual(['case']);
  });

  it('accepts a file exactly at its limit and refuses one byte more', async () => {
    const exact = [...ZIP, ...new Array<number>(28).fill(7)];
    const atLimit = await readDroppedFiles([blob(exact)], SMALL);
    expect(atLimit.map((entry) => entry.kind)).toEqual(['case']);
    const over = await readDroppedFiles([blob([...exact, 7])], SMALL);
    expect(over).toEqual([{ kind: 'refused', code: 'case-too-large' }]);
  });
});

describe('property', () => {
  it('a file is a case file exactly when its first four bytes are the zip signature', async () => {
    await fc.assert(
      fc.asyncProperty(fc.uint8Array({ minLength: 1, maxLength: 64 }), async (values) => {
        const bytes = [...values];
        const [entry] = await readDroppedFiles([blob(bytes)]);
        const isCase = bytes.length >= 4 && ZIP.every((byte, index) => bytes[index] === byte);
        expect(entry?.kind).toBe(isCase ? 'case' : 'film');
      }),
    );
  });
});
