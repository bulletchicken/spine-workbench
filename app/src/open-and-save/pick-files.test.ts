import { afterEach, describe, expect, it, vi } from 'vitest';
import { pickCaseFile, pickFilms } from './pick-files';

/** A stand-in for the file input: `choose` plays its change or cancel event. */
function fakeDialog(outcome: { files: File[] } | 'cancel') {
  const listeners: Record<string, () => void> = {};
  const input = {
    type: '',
    multiple: false,
    files: null as FileList | null,
    addEventListener(name: string, listener: () => void) {
      listeners[name] = listener;
    },
    click() {
      if (outcome === 'cancel') {
        listeners.cancel?.();
        return;
      }
      input.files = outcome.files as unknown as FileList;
      listeners.change?.();
    },
  };
  vi.stubGlobal('document', { createElement: () => input });
  return input;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('pickers', () => {
  it('a films picker is a file input that takes several files, and tells films from case files by their bytes', async () => {
    const input = fakeDialog({
      files: [new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], 'x.png'), new File([new Uint8Array([0x50, 0x4b, 0x03, 0x04])], 'y.dat')],
    });
    const out = await pickFilms();
    expect(input.type).toBe('file');
    expect(input.multiple).toBe(true);
    expect(out.map((entry) => entry.kind)).toEqual(['film', 'case']);
  });

  it('a case file picker is a file input that takes one file', async () => {
    const input = fakeDialog({ files: [new File([new Uint8Array([0x50, 0x4b, 0x03, 0x04, 1])], 'case.bin')] });
    const out = await pickCaseFile();
    expect(input.type).toBe('file');
    expect(input.multiple).toBe(false);
    expect(out.map((entry) => entry.kind)).toEqual(['case']);
  });

  it('a cancelled dialog resolves with nothing, changing nothing', async () => {
    fakeDialog('cancel');
    expect(await pickFilms()).toEqual([]);
  });
});
