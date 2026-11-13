import type { GalleryPage } from '@/gallery/gallery-page';
import type { SlotFace } from '@/look-and-feel/data-attributes';
import '../guide.gallery.css';
import { FilmsCard } from './FilmsCard';

const NAMES = ['Front', 'Side', 'Bending left', 'Bending right'];
const films = (faces: SlotFace[]) => NAMES.map((name, at) => ({ id: name, name, face: faces[at] ?? 'empty' }));

const page: GalleryPage = {
  area: 'guide',
  part: 'FilmsCard',
  states: [
    {
      name: 'Waiting for files',
      show: () => (
        <div className="guide-gallery-column">
          <FilmsCard films={films(['empty', 'empty', 'empty', 'empty'])} />
        </div>
      ),
    },
    {
      name: 'One up, three being read',
      show: () => (
        <div className="guide-gallery-column">
          <FilmsCard films={films(['hung', 'reading', 'reading', 'reading'])} />
        </div>
      ),
    },
    {
      name: 'One that could not be read, with the fault’s own words',
      show: () => (
        <div className="guide-gallery-column">
          <FilmsCard
            films={[
              ...films(['hung', 'hung', 'hung']).slice(0, 3),
              { id: 'Bending right', name: 'Bending right', face: 'bad', line: 'This image is compressed in a way the app cannot open' },
            ]}
          />
        </div>
      ),
    },
    {
      name: 'Four up',
      show: () => (
        <div className="guide-gallery-column">
          <FilmsCard films={films(['hung', 'hung', 'hung', 'hung'])} />
        </div>
      ),
    },
  ],
};

export default page;
