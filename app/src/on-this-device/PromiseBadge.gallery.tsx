import { useRef, useState, type CSSProperties } from 'react';
import { Popover } from '@/building-blocks/Popover';
import type { GalleryPage } from '@/gallery/gallery-page';
import { ON_THIS_DEVICE_WORDS } from '@/words/on-this-device';
import { PromiseBadge } from './PromiseBadge';
import { PromisePopover } from './PromisePopover';

function Badge({ startOpen }: { startOpen: boolean }) {
  const [open, setOpen] = useState(startOpen);
  const badge = useRef<HTMLButtonElement>(null);
  return (
    <div className="frame" style={{ position: 'relative', width: 360, height: open ? 260 : 48, '--col': '100%' } as CSSProperties}>
      <div style={{ display: 'flex', justifyContent: 'end' }}>
        <PromiseBadge ref={badge} open={open} onOpenChange={setOpen} />
      </div>
      <Popover anchor={badge} open={open} onClose={() => { setOpen(false); }} label={ON_THIS_DEVICE_WORDS.badge} className="note glass promise-note">
        <PromisePopover requests={{ sent: 0, blocked: 0 }} />
      </Popover>
    </div>
  );
}
const page: GalleryPage = {
  area: 'on-this-device', part: 'PromiseBadge', states: [
    { name: 'Closed: the promise with no count on the badge', show: () => <Badge startOpen={false} /> },
    { name: 'Open: Escape, a press outside or focus moving on closes it', show: () => <Badge startOpen /> },
  ],
};
export default page;
