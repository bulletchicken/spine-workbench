import { useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import './on-this-device.gallery.css';
import { PromiseBadge } from './PromiseBadge';

function Badge({ startOpen }: { startOpen: boolean }) {
  const [open, setOpen] = useState(startOpen);
  return (
    <div className="promise-gallery-place" data-closed={!startOpen || undefined}>
      <PromiseBadge requests={{ sent: 0, blocked: 0 }} open={open} onOpenChange={setOpen} />
    </div>
  );
}

const page: GalleryPage = {
  area: 'on-this-device',
  part: 'PromiseBadge',
  states: [
    { name: 'Closed: a press, or Enter with focus on it, opens the popover', show: () => <Badge startOpen={false} /> },
    { name: 'Open: Escape, a press outside or focus moving on closes it', show: () => <Badge startOpen /> },
  ],
};

export default page;
