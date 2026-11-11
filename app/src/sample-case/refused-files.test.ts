import { crc32 } from 'node:zlib';
import { describe, expect, it } from 'vitest';
import { notAFilm, REFUSED_CASE_JSON, refusedCase } from './refused-files';

/** The entries of a zip, found through its central directory as a reader finds them. */
function entriesOf(zip: Uint8Array): Map<string, Uint8Array> {
  const view = new DataView(zip.buffer, zip.byteOffset, zip.byteLength);
  const end = zip.length - 22;
  expect(view.getUint32(end, true)).toBe(0x06054b50);
  const count = view.getUint16(end + 10, true);
  let at = view.getUint32(end + 16, true);
  const entries = new Map<string, Uint8Array>();
  for (let k = 0; k < count; k++) {
    expect(view.getUint32(at, true)).toBe(0x02014b50);
    const method = view.getUint16(at + 10, true);
    const crc = view.getUint32(at + 16, true);
    const size = view.getUint32(at + 24, true);
    const nameLength = view.getUint16(at + 28, true);
    const name = String.fromCharCode(...zip.subarray(at + 46, at + 46 + nameLength));
    const local = view.getUint32(at + 42, true);
    expect(view.getUint32(local, true)).toBe(0x04034b50);
    const start = local + 30 + view.getUint16(local + 26, true) + view.getUint16(local + 28, true);
    const data = zip.subarray(start, start + size);
    expect(method).toBe(0);
    expect(crc32(data)).toBe(crc);
    entries.set(name, data);
    at += 46 + nameLength;
  }
  return entries;
}

describe('the files the app must refuse', () => {
  it('a note is not a PNG, a JPEG or a DICOM file', () => {
    const bytes = new Uint8Array(notAFilm());
    expect([...bytes.subarray(0, 4)]).not.toEqual([137, 80, 78, 71]);
    expect([...bytes.subarray(0, 2)]).not.toEqual([0xff, 0xd8]);
    expect(bytes.length).toBeLessThan(132);
  });

  it('the refused case is a zip of case.json and README.txt, with one field the format does not have', () => {
    const { file, error } = refusedCase();
    const entries = entriesOf(new Uint8Array(file));
    expect([...entries.keys()]).toEqual(['case.json', 'README.txt']);
    const text = new TextDecoder().decode(entries.get('case.json'));
    expect(text).toBe(REFUSED_CASE_JSON);
    expect(Object.keys(JSON.parse(text) as object)).toEqual(['format', 'version', 'savedWith', 'films', 'marks', 'nextVertebraId', 'notes']);
    expect(error).toMatchObject({ code: 'case-unknown-field', detail: 'notes' });
  });
});
