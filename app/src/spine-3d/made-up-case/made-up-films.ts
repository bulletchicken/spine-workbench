// The made-up case's front and side films as small pictures, for the 3D
// view's gallery. Each covers its whole film at a quarter of its size, so
// it can be laid on a sheet the film's own size. They come in as text with
// the gallery's own script and are decoded by the browser: nothing is
// fetched. Painted by the engine from made-up numbers; not a patient's.

import frontPicture from './front-film.png?inline';
import sidePicture from './side-film.png?inline';

export interface MadeUpFilms {
  front: HTMLCanvasElement;
  side: HTMLCanvasElement;
}

async function drawn(address: string): Promise<HTMLCanvasElement> {
  const picture = new Image();
  picture.src = address;
  await picture.decode();
  const canvas = document.createElement('canvas');
  canvas.width = picture.naturalWidth;
  canvas.height = picture.naturalHeight;
  canvas.getContext('2d')?.drawImage(picture, 0, 0);
  return canvas;
}

let loading: Promise<MadeUpFilms> | null = null;

/** Both films, each on a canvas of its own. The same answer every time. */
export function madeUpFilms(): Promise<MadeUpFilms> {
  loading ??= Promise.all([drawn(frontPicture), drawn(sidePicture)]).then(([front, side]) => ({ front, side }));
  return loading;
}
