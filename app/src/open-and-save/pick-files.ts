// The one picker for films and the one for a case file. Each opens the
// system's file dialog and hands what was chosen to read-dropped-files, so
// a chosen file is told apart by its bytes exactly as a dropped one is. No
// step has a picker of its own.

import { readDroppedFiles, type DroppedFile } from './read-dropped-files';

/** Open the dialog; resolves with what was chosen, or nothing when it is cancelled. */
function choose(multiple: boolean): Promise<Blob[]> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.multiple = multiple;
    input.addEventListener(
      'change',
      () => {
        resolve(Array.from(input.files ?? []));
      },
      { once: true },
    );
    input.addEventListener(
      'cancel',
      () => {
        resolve([]);
      },
      { once: true },
    );
    input.click();
  });
}

/** Choose any number of films (or case files, which are told apart by their bytes). */
export async function pickFilms(): Promise<DroppedFile[]> {
  return readDroppedFiles(await choose(true));
}

/** Choose one case file. */
export async function pickCaseFile(): Promise<DroppedFile[]> {
  return readDroppedFiles(await choose(false));
}
