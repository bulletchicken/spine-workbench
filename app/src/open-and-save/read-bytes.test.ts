import { describe, expect, it } from 'vitest';
import { EngineFailure } from '@/engine-room/engine-client';
import { readBytes } from './read-bytes';

describe('reading a file in full', () => {
  it('gives back every byte of the file, in order', async () => {
    const bytes = await readBytes(new Blob([new Uint8Array([0x89, 0x50, 0x4e, 0x47]), new Uint8Array([1, 2])]));
    expect([...new Uint8Array(bytes)]).toEqual([0x89, 0x50, 0x4e, 0x47, 1, 2]);
  });

  it('refuses a file the system will not give as film-truncated, the code the drop uses', async () => {
    const moved = { arrayBuffer: () => Promise.reject(new DOMException('the file was moved', 'NotReadableError')) } as unknown as Blob;
    const refusal = readBytes(moved);
    await expect(refusal).rejects.toBeInstanceOf(EngineFailure);
    await expect(refusal).rejects.toMatchObject({ error: { code: 'film-truncated', detail: null } });
  });
});
