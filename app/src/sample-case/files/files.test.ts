import { crc32 as nodeCrc32, inflateSync } from 'node:zlib';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { adler32, crc32 } from './checksums';
import { writeDicom } from './dicom-file';
import { halfFloatBits } from './half-float';
import { storedZlib, writePng } from './png-file';

const ascii = (text: string) => Uint8Array.from(text, (c) => c.charCodeAt(0));

/** PNG chunks, read the way the specification lays them out. */
function chunksOf(png: Uint8Array) {
  const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
  const chunks: { type: string; data: Uint8Array; crc: number }[] = [];
  for (let at = 8; at < png.length; ) {
    const length = view.getUint32(at);
    const type = String.fromCharCode(...png.subarray(at + 4, at + 8));
    chunks.push({ type, data: png.subarray(at + 8, at + 8 + length), crc: view.getUint32(at + 8 + length) });
    at += 12 + length;
  }
  return chunks;
}

/** A half float's value, from its bits. */
function halfValue(bits: number): number {
  const sign = bits & 0x8000 ? -1 : 1;
  const exponent = (bits >> 10) & 0x1f;
  const fraction = bits & 0x3ff;
  if (exponent === 0) return sign * fraction * 2 ** -24;
  if (exponent === 31) return fraction === 0 ? sign * Infinity : Number.NaN;
  return sign * (1 + fraction / 1024) * 2 ** (exponent - 15);
}

describe('checksums', () => {
  it('give the published check values', () => {
    expect(crc32(ascii('123456789'))).toBe(0xcbf43926);
    expect(adler32(ascii('Wikipedia'))).toBe(0x11e60398);
    expect(adler32(new Uint8Array(0))).toBe(1);
  });

  it('agree with zlib on any bytes', () => {
    fc.assert(
      fc.property(fc.uint8Array({ maxLength: 20000 }), (bytes) => {
        expect(crc32(bytes)).toBe(nodeCrc32(bytes));
      }),
      { numRuns: 50 },
    );
  });
});

describe('PNG files', () => {
  it('inflate back to every row, each filtered with none', () => {
    const width = 300;
    const height = 260; // more than 65535 bytes of rows, so several stored blocks
    const grey = Uint8Array.from({ length: width * height }, (_, k) => (k * 7) % 251);
    const png = writePng(grey, width, height, { keyword: 'Title', value: 'a test film' });
    expect([...png.subarray(0, 8)]).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
    const chunks = chunksOf(png);
    expect(chunks.map((c) => c.type)).toEqual(['IHDR', 'tEXt', 'IDAT', 'IEND']);
    for (const chunk of chunks) expect(chunk.crc).toBe(nodeCrc32(Buffer.concat([ascii(chunk.type), chunk.data])));
    const header = new DataView(chunks[0]?.data.buffer ?? new ArrayBuffer(13), chunks[0]?.data.byteOffset ?? 0, 13);
    expect([header.getUint32(0), header.getUint32(4), header.getUint8(8), header.getUint8(9)]).toEqual([width, height, 8, 0]);
    expect(String.fromCharCode(...(chunks[1]?.data ?? []))).toBe('Title\0a test film');
    const rows = inflateSync(chunks[2]?.data ?? new Uint8Array(0));
    expect(rows.length).toBe(height * (width + 1));
    for (let row = 0; row < height; row++) {
      expect(rows[row * (width + 1)]).toBe(0);
      expect(rows.subarray(row * (width + 1) + 1, (row + 1) * (width + 1)).equals(grey.subarray(row * width, (row + 1) * width))).toBe(true);
    }
  });

  it('stores an empty stream as one final block', () => {
    expect(inflateSync(storedZlib(new Uint8Array(0))).length).toBe(0);
  });
});

describe('DICOM files', () => {
  /** Elements of an explicit VR little endian stream, walked from `at`. */
  function elementsOf(bytes: Uint8Array, at: number) {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const found = new Map<string, { vr: string; value: Uint8Array }>();
    while (at < bytes.length) {
      const tag = `${view.getUint16(at, true).toString(16).padStart(4, '0')},${view.getUint16(at + 2, true).toString(16).padStart(4, '0')}`;
      const vr = String.fromCharCode(bytes[at + 4] ?? 0, bytes[at + 5] ?? 0);
      const long = ['OB', 'OW', 'OF', 'SQ', 'UT', 'UN'].includes(vr);
      const length = long ? view.getUint32(at + 8, true) : view.getUint16(at + 6, true);
      const start = at + (long ? 12 : 8);
      found.set(tag, { vr, value: bytes.subarray(start, start + length) });
      at = start + length;
    }
    return found;
  }
  const textOf = (value: Uint8Array | undefined) => String.fromCharCode(...(value ?? [])).replace(/[\0 ]+$/, '');

  it('is a part 10 file whose header and pixels a reader can find', () => {
    const grey = Uint8Array.of(0, 10, 20, 30, 40, 250);
    const file = writeDicom({ grey, columns: 3, rows: 2, mmPerPx: 0.45, viewPosition: 'LL', windowing: { centre: 125, width: 250 }, uid: '42' });
    expect(file.subarray(0, 128).every((byte) => byte === 0)).toBe(true);
    expect(textOf(file.subarray(128, 132))).toBe('DICM');
    const all = elementsOf(file, 132);
    const lengthBytes = Uint8Array.from(all.get('0002,0000')?.value ?? []);
    const groupLength = new DataView(lengthBytes.buffer).getUint32(0, true);
    const firstData = 132 + 12 + groupLength;
    expect(file[firstData]).toBe(0x08); // the group after the file meta starts right where its length says
    expect(textOf(all.get('0002,0010')?.value)).toBe('1.2.840.10008.1.2.1');
    expect(textOf(all.get('0008,0016')?.value)).toBe('1.2.840.10008.5.1.4.1.1.1.1');
    expect(textOf(all.get('0008,0018')?.value)).toBe('2.25.42');
    expect(textOf(all.get('0008,0060')?.value)).toBe('DX');
    expect(textOf(all.get('0018,5101')?.value)).toBe('LL');
    expect(textOf(all.get('0028,0004')?.value)).toBe('MONOCHROME2');
    expect(textOf(all.get('0028,0030')?.value)).toBe('0.45\\0.45');
    expect(textOf(all.get('0028,0a02')?.value)).toBe('GEOMETRY');
    const us = (tag: string) => new DataView(Uint8Array.from(all.get(tag)?.value ?? []).buffer).getUint16(0, true);
    expect([us('0028,0010'), us('0028,0011'), us('0028,0100'), us('0028,0101'), us('0028,0102')]).toEqual([2, 3, 8, 8, 7]);
    expect([...(all.get('7fe0,0010')?.value ?? [])]).toEqual([...grey]);
    for (const { value } of all.values()) expect(value.length % 2).toBe(0);
  });

  it('pads pixel data of odd length with one zero byte', () => {
    const grey = Uint8Array.of(9, 8, 7);
    const file = writeDicom({ grey, columns: 3, rows: 1, mmPerPx: 0.45, viewPosition: 'AP', windowing: { centre: 8, width: 2 }, uid: '7' });
    expect([...(elementsOf(file, 132).get('7fe0,0010')?.value ?? [])]).toEqual([9, 8, 7, 0]);
    expect(file.length % 2).toBe(0);
  });
});

describe('half floats', () => {
  it('match values worked out by hand', () => {
    expect(halfFloatBits(0)).toBe(0x0000);
    expect(halfFloatBits(1)).toBe(0x3c00);
    expect(halfFloatBits(0.5)).toBe(0x3800);
    expect(halfFloatBits(-2)).toBe(0xc000);
    expect(halfFloatBits(65504)).toBe(0x7bff);
    expect(halfFloatBits(65520)).toBe(0x7c00); // halfway to the next step, which is infinity
    expect(halfFloatBits(2 ** -14)).toBe(0x0400);
    expect(halfFloatBits(2 ** -24)).toBe(0x0001);
    expect(halfFloatBits(2 ** -25)).toBe(0x0000); // a tie, to the even zero
    expect(halfFloatBits(1 / 3)).toBe(0x3555);
    // 1/255 = 2^-8 × 1.0039…, and 0.0039 × 1024 rounds to 4.
    expect(halfFloatBits(1 / 255)).toBe(0x1c04);
  });

  it('round to within half a step', () => {
    fc.assert(
      fc.property(fc.double({ min: 2 ** -14, max: 65000, noNaN: true }), (value) => {
        expect(Math.abs(halfValue(halfFloatBits(value)) - value)).toBeLessThanOrEqual(value * 2 ** -11 * 1.0001);
      }),
    );
  });
});
