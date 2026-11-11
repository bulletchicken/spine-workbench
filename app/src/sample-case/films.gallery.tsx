// The four sample films on plain canvases, side by side as the bays of
// the mock's "Four films up" hang them, to be compared with 04-up. As
// stored, bending left is upside down; turned upright it lines up with
// the mock.

import { useEffect, useRef } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import type { FilmKind } from '@/shapes';
import { FILM_HEIGHT, FILM_KINDS, FILM_WIDTH } from './anatomy';
import { storedGrey } from './films';
import './films.gallery.css';

const NAMES: Record<FilmKind, string> = { front: 'Front', side: 'Side', 'bend-left': 'Bending left', 'bend-right': 'Bending right' };

function FilmCanvas({ kind, upright }: { kind: FilmKind; upright: boolean }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const context = canvas.current?.getContext('2d');
    if (context === null || context === undefined) return;
    const grey = storedGrey(kind);
    const turn = upright && kind === 'bend-left';
    const picture = context.createImageData(FILM_WIDTH, FILM_HEIGHT);
    for (let k = 0; k < grey.length; k++) {
      const value = grey[turn ? grey.length - 1 - k : k] ?? 0;
      picture.data[4 * k] = value;
      picture.data[4 * k + 1] = value;
      picture.data[4 * k + 2] = value;
      picture.data[4 * k + 3] = 255;
    }
    context.putImageData(picture, 0, 0);
  }, [kind, upright]);
  return (
    <figure className="sample-film">
      <figcaption>{NAMES[kind]}</figcaption>
      <canvas ref={canvas} width={FILM_WIDTH} height={FILM_HEIGHT} />
    </figure>
  );
}

function FourFilms({ upright }: { upright: boolean }) {
  return (
    <div className="sample-films">
      {FILM_KINDS.map((kind) => (
        <FilmCanvas key={kind} kind={kind} upright={upright} />
      ))}
    </div>
  );
}

const page: GalleryPage = {
  area: 'sample-case',
  part: 'films',
  states: [
    { name: 'Turned upright, as the bays of 04-up show them', show: () => <FourFilms upright /> },
    { name: 'As stored: bending left upside down', show: () => <FourFilms upright={false} /> },
  ],
};

export default page;
