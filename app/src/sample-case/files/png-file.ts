// A real 8-bit greyscale PNG, written by hand: the signature, a header, an
// optional text chunk, the pixels as one zlib stream of stored (not
// compressed) blocks, and the end (PNG specification, third edition;
// RFC 1950 and 1951 for the stream). Every row is filtered with "none".

import { adler32, crc32 } from './checksums';

const SIGNATURE = [137, 80, 78, 71, 13, 10, 26, 10];
const STORED_BLOCK = 65535;

function chunk(type: string, data: Uint8Array): Uint8Array {
  const out = new Uint8Array(12 + data.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, data.length);
  for (let k = 0; k < 4; k++) out[4 + k] = type.charCodeAt(k);
  out.set(data, 8);
  view.setUint32(8 + data.length, crc32(out.subarray(4, 8 + data.length)));
  return out;
}

/** The bytes as a zlib stream of stored blocks. */
export function storedZlib(raw: Uint8Array): Uint8Array {
  const blocks = Math.max(1, Math.ceil(raw.length / STORED_BLOCK));
  const out = new Uint8Array(2 + blocks * 5 + raw.length + 4);
  out[0] = 0x78; // deflate, a 32 KiB window
  out[1] = 0x01; // no dictionary, the fastest level; 0x7801 is a multiple of 31
  let at = 2;
  for (let block = 0; block < blocks; block++) {
    const start = block * STORED_BLOCK;
    const length = Math.min(STORED_BLOCK, raw.length - start);
    out[at] = block === blocks - 1 ? 1 : 0; // the last block, stored
    out[at + 1] = length & 0xff;
    out[at + 2] = length >>> 8;
    out[at + 3] = ~length & 0xff;
    out[at + 4] = (~length >>> 8) & 0xff;
    out.set(raw.subarray(start, start + length), at + 5);
    at += 5 + length;
  }
  new DataView(out.buffer).setUint32(at, adler32(raw));
  return out;
}

const latin1 = (text: string) => Uint8Array.from(text, (character) => character.charCodeAt(0) & 0xff);

/**
 * An 8-bit greyscale PNG of `grey` (row-major, `width` × `height`), with
 * one tEXt chunk when `text` is given.
 */
export function writePng(grey: Uint8Array, width: number, height: number, text?: { keyword: string; value: string }): Uint8Array {
  const header = new Uint8Array(13);
  const headerView = new DataView(header.buffer);
  headerView.setUint32(0, width);
  headerView.setUint32(4, height);
  header[8] = 8; // bits per sample
  header[9] = 0; // greyscale; compression, filter and interlace stay 0

  const raw = new Uint8Array(height * (width + 1));
  for (let row = 0; row < height; row++) raw.set(grey.subarray(row * width, (row + 1) * width), row * (width + 1) + 1);

  const parts = [Uint8Array.from(SIGNATURE), chunk('IHDR', header)];
  if (text !== undefined) {
    const keyword = latin1(text.keyword);
    const value = latin1(text.value);
    const data = new Uint8Array(keyword.length + 1 + value.length);
    data.set(keyword, 0);
    data.set(value, keyword.length + 1);
    parts.push(chunk('tEXt', data));
  }
  parts.push(chunk('IDAT', storedZlib(raw)), chunk('IEND', new Uint8Array(0)));

  const out = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}
