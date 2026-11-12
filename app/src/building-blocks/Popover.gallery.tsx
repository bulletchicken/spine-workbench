import { useRef, useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { Button } from './Button';
import { Popover } from './Popover';
import './Popover.gallery.css';

/**
 * A control and its popover, wearing the design system's trust note (`note glass`) as it is
 * drawn there: a sheet as wide as the bar, the lead line and the count on the left, the credit on
 * the right behind a hairline.
 */
function Badge({ startOpen }: { startOpen: boolean }) {
  const [open, setOpen] = useState(startOpen);
  const badge = useRef<HTMLButtonElement>(null);
  return (
    <div className="blocks-gallery-pop-room" data-align="end" style={{ width: 'var(--col-max)', height: 'auto' }}>
      <div className="blocks-gallery-anchor" style={{ flexDirection: 'column', alignItems: 'flex-end', gap: 'var(--space-8)' }}>
        <Button
          ref={badge}
          look="text"
          aria-haspopup="dialog"
          aria-expanded={open}
          onPress={() => {
            setOpen(!open);
          }}
        >
          On this device only
        </Button>
        <Popover
          open={open}
          onClose={() => {
            setOpen(false);
          }}
          anchor={badge}
          label="On this device only"
          className="note glass"
        >
          <div>
            <p className="note-lead">Your films never leave this computer.</p>
            <p className="note-count">
              Requests sent: <span className="num">0</span>
            </p>
          </div>
          <p className="note-credit">Bone shapes: BodyParts3D, © The Database Center for Life Science licensed under CC Attribution 4.0 International.</p>
        </Popover>
      </div>
    </div>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Popover',
  states: [
    { name: 'Open: the note fades in and comes down from under its control', show: () => <Badge startOpen /> },
    { name: 'Closed: on the page, unseen, inert; press the control to open it', show: () => <Badge startOpen={false} /> },
  ],
};

export default page;
