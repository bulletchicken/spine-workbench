// The byte payloads inside engine answers and watch frames: the views the
// app reads them through, and the buffers a worker hands over in its
// transfer list.
//
// Byte fields cross as fresh `Uint8Array`s. Float payloads inside them
// (a barcode's values, a strip's maps) are little-endian `f32`s, and a
// strip block's validity is one bit per cell.

import { EngineFailure } from './engine-client';

const LITTLE_ENDIAN = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;

/**
 * The little-endian `f32`s in `bytes`. A view over the same memory when
 * the platform is little-endian and the bytes start on a four-byte
 * boundary; a copy otherwise.
 */
export function floatsIn(bytes: Uint8Array): Float32Array {
  if (bytes.byteLength % 4 !== 0) {
    throw new EngineFailure({
      code: 'engine-out-of-sync',
      message: 'a float payload is not a whole number of f32s',
      detail: `${String(bytes.byteLength)} bytes`,
    });
  }
  const count = bytes.byteLength / 4;
  if (LITTLE_ENDIAN && bytes.byteOffset % 4 === 0) return new Float32Array(bytes.buffer, bytes.byteOffset, count);
  const reader = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const floats = new Float32Array(count);
  for (let i = 0; i < count; i += 1) floats[i] = reader.getFloat32(i * 4, true);
  return floats;
}

/** Whether cell `cell` is usable in a validity mask: one bit per cell, row-major, lowest bit first. */
export function isUsable(valid: Uint8Array, cell: number): boolean {
  const byte = valid[cell >> 3];
  if (byte === undefined) return false;
  return ((byte >> (cell & 7)) & 1) === 1;
}

/**
 * Every buffer behind the typed arrays (and bare buffers) inside `value`,
 * each once: the transfer list that hands them over without a copy.
 */
export function transferList(value: unknown): ArrayBuffer[] {
  const buffers = new Set<ArrayBuffer>();
  const seen = new Set<object>();
  const visit = (item: unknown): void => {
    if (typeof item !== 'object' || item === null || seen.has(item)) return;
    seen.add(item);
    if (item instanceof ArrayBuffer) {
      buffers.add(item);
    } else if (ArrayBuffer.isView(item)) {
      if (item.buffer instanceof ArrayBuffer) buffers.add(item.buffer);
    } else if (Array.isArray(item)) {
      for (const element of item) visit(element);
    } else {
      for (const field of Object.values(item)) visit(field);
    }
  };
  visit(value);
  return [...buffers];
}
