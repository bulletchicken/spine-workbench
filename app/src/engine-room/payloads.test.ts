import { describe, expect, it } from 'vitest';
import { EngineFailure } from './engine-client';
import { floatsIn, isUsable, transferList } from './payloads';

describe('float payloads', () => {
  it('reads little-endian f32s worked out by hand', () => {
    // 1.0 = 0x3F800000, −2.5 = 0xC0200000, 0.15625 = 0x3E200000, each written lowest byte first.
    const bytes = new Uint8Array([0x00, 0x00, 0x80, 0x3f, 0x00, 0x00, 0x20, 0xc0, 0x00, 0x00, 0x20, 0x3e]);
    expect([...floatsIn(bytes)]).toEqual([1, -2.5, 0.15625]);
  });

  it('reads bytes that start part-way into their buffer', () => {
    const buffer = new Uint8Array([0xff, 0x00, 0x00, 0x80, 0x3f]);
    expect([...floatsIn(buffer.subarray(1))]).toEqual([1]);
  });

  it('refuses a payload that is not whole f32s', () => {
    expect(() => floatsIn(new Uint8Array(6))).toThrow(EngineFailure);
  });
});

describe('validity bits', () => {
  it('reads one bit per cell, lowest bit first', () => {
    const valid = new Uint8Array([0b0000_0101, 0b1000_0000]);
    expect([0, 1, 2, 3, 7, 8, 14, 15].map((cell) => isUsable(valid, cell))).toEqual([
      true,
      false,
      true,
      false,
      false,
      false,
      false,
      true,
    ]);
  });

  it('counts a cell past the mask as unusable', () => {
    expect(isUsable(new Uint8Array([0xff]), 8)).toBe(false);
  });
});

describe('transfer lists', () => {
  it('names each buffer inside a value once', () => {
    const shared = new Uint8Array(4);
    const tile = new Uint8Array(8);
    const pixels = new Uint16Array(2);
    const value = { tile: { data: tile }, again: shared, list: [shared, pixels], bare: shared.buffer, note: 'x', n: 3 };
    const list = transferList(value);
    expect(list).toHaveLength(3);
    expect(new Set(list)).toEqual(new Set([shared.buffer, tile.buffer, pixels.buffer]));
  });

  it('is empty for a value with no bytes', () => {
    expect(transferList({ a: [1, 2], b: null, c: 'text' })).toEqual([]);
  });
});
