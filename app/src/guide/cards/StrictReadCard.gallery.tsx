import type { GalleryPage } from '@/gallery/gallery-page';
import type { RowState } from '@/look-and-feel/data-attributes';
import '../guide.gallery.css';
import { StrictReadCard } from './StrictReadCard';

const LINES = ['It is a spine-case file, version 1', 'Four films under their fixed names', 'The data file matches the format exactly', '17 vertebrae, pins and L5 as saved'];
const lines = (states: RowState[]) => LINES.map((line, at) => ({ line, state: states[at] ?? 'wait' }));

const page: GalleryPage = {
  area: 'guide',
  part: 'StrictReadCard',
  states: [
    {
      name: 'Two lines checked, the third being read',
      show: () => (
        <div className="guide-gallery-column">
          <StrictReadCard lines={lines(['ok', 'ok', 'busy', 'wait'])} />
        </div>
      ),
    },
    {
      name: 'Every line checked',
      show: () => (
        <div className="guide-gallery-column">
          <StrictReadCard lines={lines(['ok', 'ok', 'ok', 'ok'])} />
        </div>
      ),
    },
    {
      name: 'Refused at the third line: nothing after it is ticked',
      show: () => (
        <div className="guide-gallery-column">
          <StrictReadCard lines={lines(['ok', 'ok', 'bad', 'wait'])} />
        </div>
      ),
    },
  ],
};

export default page;
