import type { GalleryPage } from '@/gallery/gallery-page';
import '../guide.gallery.css';
import { FilmFactsCard } from './FilmFactsCard';

const SIZE = '0.45 mm per pixel, from the file';

const page: GalleryPage = {
  area: 'guide',
  part: 'FilmFactsCard',
  states: [
    {
      name: 'Two films waiting for a person',
      show: () => (
        <div className="guide-gallery-column">
          <FilmFactsCard
            films={[
              { id: 'front', name: 'Front', state: 'warn', facts: [SIZE, 'Left and right: waiting for you'] },
              { id: 'side', name: 'Side', state: 'ok', facts: [SIZE, 'Faces right, read from the bulge of the upper back'] },
              { id: 'bend-left', name: 'Bending left', state: 'ok', facts: [SIZE, 'It came in upside-down and was turned upright'] },
              { id: 'bend-right', name: 'Bending right', state: 'warn', facts: ['No size in the file', 'Upright'] },
            ]}
          />
        </div>
      ),
    },
    {
      name: 'All set, two by a person',
      show: () => (
        <div className="guide-gallery-column">
          <FilmFactsCard
            films={[
              { id: 'front', name: 'Front', state: 'you', facts: [SIZE, 'Patient’s left is on the right of the picture, confirmed by you'] },
              { id: 'side', name: 'Side', state: 'ok', facts: [SIZE, 'Faces right, read from the bulge of the upper back'] },
              { id: 'bend-left', name: 'Bending left', state: 'ok', facts: [SIZE, 'It came in upside-down and was turned upright'] },
              { id: 'bend-right', name: 'Bending right', state: 'you', facts: ['0.45 mm per pixel, from your two clicks on a 25 mm ball', 'Upright'] },
            ]}
          />
        </div>
      ),
    },
  ],
};

export default page;
