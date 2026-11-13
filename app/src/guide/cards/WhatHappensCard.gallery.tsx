import type { GalleryPage } from '@/gallery/gallery-page';
import '../guide.gallery.css';
import { WhatHappensCard } from './WhatHappensCard';

const page: GalleryPage = {
  area: 'guide',
  part: 'WhatHappensCard',
  states: [
    {
      name: 'Kept, dropped, sent anywhere: never',
      show: () => (
        <div className="guide-gallery-column">
          <WhatHappensCard />
        </div>
      ),
    },
  ],
};

export default page;
