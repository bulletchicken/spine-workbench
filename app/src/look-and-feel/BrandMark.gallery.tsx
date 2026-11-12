import type { CSSProperties } from 'react';
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
        <div className="brand">
          <BrandMark />
          Spine Workbench
        </div>
      ),
    },
    { name: 'Large, to check the drawing', show: () => <div className="brand" style={{ '--icon-medium': '104px' } as CSSProperties}><BrandMark /></div> },
  ],
};

export default page;
