import { useState } from 'react';
import { Button } from '@/building-blocks/Button';
import type { GalleryPage } from '@/gallery/gallery-page';
import '../guide.gallery.css';
import { NarrationCard } from './NarrationCard';

const LINES = [
  { title: 'Looking for bone', line: 'Edges, ridges and pedicle rings are picked out on both films. Ribs are edges too.' },
  { title: 'Tracing the centre of the spine', line: 'The cheapest path through that evidence, neck to pelvis.' },
  { title: 'Straightening the spine', line: 'Unrolled into a strip, bone and disc gaps read like a barcode.' },
  { title: 'Searching for all 17 at once', line: 'One sweep weighs every way of placing them. Both films vote. Dashed outlines are guesses, not answers.' },
  { title: 'Locking in and naming', line: 'The best chain is only known at the end, so it is read back from the pelvis up.' },
  { title: 'Rating how sure it is', line: 'Blue is sure. Yellow is worth a look. Coral needs you.' },
];

/** The card stepped through its beats by hand, to watch a line open and the one before it tick. */
function Stepped() {
  const [live, setLive] = useState(0);
  return (
    <div className="guide-gallery-column">
      <NarrationCard lines={LINES} live={live < LINES.length ? live : null} finished={live} />
      <span className="gallery-note">
        <Button
          onPress={() => {
            setLive((at) => (at + 1) % (LINES.length + 1));
          }}
        >
          Next beat
        </Button>
      </span>
    </div>
  );
}

const page: GalleryPage = {
  area: 'guide',
  part: 'NarrationCard',
  states: [
    {
      name: 'Before the first beat',
      show: () => (
        <div className="guide-gallery-column">
          <NarrationCard lines={LINES} live={null} finished={0} />
        </div>
      ),
    },
    {
      name: 'The fourth beat live, three finished',
      show: () => (
        <div className="guide-gallery-column">
          <NarrationCard lines={LINES} live={3} finished={3} />
        </div>
      ),
    },
    {
      name: 'Every beat finished',
      show: () => (
        <div className="guide-gallery-column">
          <NarrationCard lines={LINES} live={null} finished={6} />
        </div>
      ),
    },
    { name: 'Stepped through by hand', show: () => <Stepped /> },
  ],
};

export default page;
