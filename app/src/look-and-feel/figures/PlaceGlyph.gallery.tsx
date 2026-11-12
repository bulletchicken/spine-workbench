import type { FilmKind } from '@/shapes';
import type { GalleryPage } from '@/gallery/gallery-page';
import { FILM_NAMES } from '@/words/film-names';
import { PlaceGlyph } from './PlaceGlyph';

const kinds: readonly FilmKind[] = ['front', 'side', 'bend-left', 'bend-right'];
function Places({ held = false }: { held?: boolean }) {
  return (
    <div data-drag={held || undefined}>
      <div className="dropzone" style={{ display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0, 1fr))', gap: 'var(--tile-gap)', maxWidth: 'var(--col-max)' }}>
        {kinds.map((kind) => (
          <figure key={kind}>
            <button className="place" aria-label={`Choose the ${FILM_NAMES[kind]} film`} style={{ width: '100%', aspectRatio: 'var(--tile-aspect)' }}><PlaceGlyph kind={kind} /></button>
            <figcaption className="t-caption" style={{ textAlign: 'center', marginTop: 'var(--caption-gap)', color: 'var(--ink-2)' }}>{FILM_NAMES[kind]}</figcaption>
          </figure>
        ))}
      </div>
    </div>
  );
}
const page: GalleryPage = {
  area: 'look-and-feel', part: 'PlaceGlyph', states: [
    { name: 'Seven blocks for each film’s curve; hover and press change all four places', show: () => <Places /> },
    { name: 'Files held over the window', show: () => <Places held /> },
  ],
};
export default page;
