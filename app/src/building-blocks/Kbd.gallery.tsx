import type { GalleryPage } from '@/gallery/gallery-page';
import { Kbd } from './Kbd';
import './Kbd.gallery.css';

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Kbd',
  states: [
    {
      name: 'Chip',
      show: () => (
        <>
          <Kbd keys={['Enter']} />
          <Kbd keys={['Escape']} />
          <Kbd keys={['Space']} />
          <Kbd keys={['ArrowLeft']} />
          <Kbd keys={['ArrowRight']} />
          <Kbd keys={['a']} />
          <Kbd keys={['b']} />
          <Kbd keys={['0']} />
          <Kbd keys={['+']} />
          <Kbd keys={['-']} />
        </>
      ),
    },
    {
      name: 'Keys held together, as this keyboard prints them',
      show: () => (
        <>
          <Kbd keys={['Mod', 'z']} />
          <Kbd keys={['Mod', 'Shift', 'z']} />
          <Kbd keys={['Mod', 's']} />
        </>
      ),
    },
    {
      name: 'Bare, beside a word',
      show: () => (
        <>
          <span className="blocks-gallery-word">
            Undo <Kbd keys={['Mod', 'z']} look="bare" />
          </span>
          <span className="blocks-gallery-word">
            Pins <Kbd keys={['v']} look="bare" />
          </span>
        </>
      ),
    },
    {
      name: 'On a lit surface, which sets its two colours',
      show: () => (
        <span className="blocks-gallery-lit">
          Choose films <Kbd keys={['Enter']} />
        </span>
      ),
    },
  ],
};

export default page;
