import type { GalleryPage } from '@/gallery/gallery-page';
import { BrandMark } from './BrandMark';
import './look-and-feel.gallery.css';

const page: GalleryPage = {
  area: 'look-and-feel',
  part: 'BrandMark',
  states: [
    {
      name: 'Beside the name, as in the top bar',
      show: () => (
        <span className="look-gallery-brand">
          <BrandMark />
          Spine Workbench
        </span>
      ),
    },
    { name: 'Large, to check the drawing', show: () => <BrandMark className="look-gallery-brand-large" /> },
  ],
};

export default page;
