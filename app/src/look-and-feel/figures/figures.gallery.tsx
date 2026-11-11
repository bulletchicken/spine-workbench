import type { GalleryPage } from '@/gallery/gallery-page';
import type { FilmKind } from '@/shapes';
import '../look-and-feel.gallery.css';
import { FIGURE_FOR_VIEW } from './figure-for-view';

const VIEWS: readonly { view: FilmKind; caption: string }[] = [
  { view: 'front', caption: 'Standing, from the front' },
  { view: 'side', caption: 'Standing, from the side' },
  { view: 'bend-left', caption: 'Leaning to the left' },
  { view: 'bend-right', caption: 'Leaning to the right' },
];

function Slots({ size, lit }: { size: 'full' | 'small'; lit?: boolean }) {
  return (
    <div className="look-gallery-slots" data-size={size}>
      {VIEWS.map(({ view, caption }, index) => {
        const Figure = FIGURE_FOR_VIEW[view];
        return (
          <div key={view} className="look-gallery-slot" data-lit={lit === true && index === 0 ? 'yes' : 'no'}>
            <Figure />
            <span>{caption}</span>
          </div>
        );
      })}
    </div>
  );
}

const page: GalleryPage = {
  area: 'look-and-feel',
  part: 'View figures',
  states: [
    { name: 'The four views, in empty slots, the first under the pointer', show: () => <Slots size="full" lit /> },
    { name: 'In narrow slots, as under 1200', show: () => <Slots size="small" /> },
  ],
};

export default page;
