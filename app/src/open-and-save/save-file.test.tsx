import { afterEach, expect, it, vi } from 'vitest';
import { chooseCaseDestination } from './save-file';

afterEach(() => { delete window.showSaveFilePicker; vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.useRealTimers(); });

it('chooses the native destination first, writes local bytes, and waits for close', async () => {
  const write = vi.fn(() => Promise.resolve());
  const close = vi.fn(() => Promise.resolve());
  const abort = vi.fn(() => Promise.resolve());
  const createWritable = vi.fn(() => Promise.resolve({ write, close, abort }));
  const picker = vi.fn(() => Promise.resolve({ createWritable }));
  window.showSaveFilePicker = picker;
  const choosing = chooseCaseDestination();
  expect(picker).toHaveBeenCalledExactlyOnceWith({ suggestedName: 'spine-case.spinecase' });
  expect(createWritable).not.toHaveBeenCalled();
  const save = await choosing;
  expect(await save(new Uint8Array([80, 75, 3, 4]))).toBe('written');
  expect(write).toHaveBeenCalledExactlyOnceWith(expect.any(Blob));
  expect(close).toHaveBeenCalledTimes(1);
  expect(abort).not.toHaveBeenCalled();
});

it('downloads a local blob with a fixed name when the native dialog is unavailable', async () => {
  vi.useFakeTimers();
  const createObjectURL = vi.fn(() => 'blob:local-case');
  const revokeObjectURL = vi.fn();
  vi.stubGlobal('URL', { createObjectURL, revokeObjectURL });
  let download: { href: string; name: string } | null = null;
  vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) {
    download = { href: this.href, name: this.download };
  });
  const save = await chooseCaseDestination();
  expect(await save(new Uint8Array([80, 75, 3, 4]))).toBe('download');
  expect(createObjectURL).toHaveBeenCalledExactlyOnceWith(expect.any(Blob));
  expect(download).toEqual({ href: 'blob:local-case', name: 'spine-case.spinecase' });
  expect(document.querySelector('a[download]')).toBeNull();
  await vi.advanceTimersByTimeAsync(59_999);
  expect(revokeObjectURL).not.toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(1);
  expect(revokeObjectURL).toHaveBeenCalledWith('blob:local-case');
});


it.each(['write', 'close'] as const)('aborts an unfinished native file when %s fails and preserves the original error', async (stage) => {
  const error = new DOMException('destination unavailable', 'NotAllowedError');
  const abort = vi.fn(() => Promise.reject(new DOMException('already closed', 'InvalidStateError')));
  const write = vi.fn(() => stage === 'write' ? Promise.reject(error) : Promise.resolve());
  const close = vi.fn(() => stage === 'close' ? Promise.reject(error) : Promise.resolve());
  window.showSaveFilePicker = () => Promise.resolve({ createWritable: () => Promise.resolve({ write, close, abort }) });
  const save = await chooseCaseDestination();
  await expect(save(new Uint8Array([80, 75, 3, 4]))).rejects.toBe(error);
  expect(abort).toHaveBeenCalledTimes(1);
});
