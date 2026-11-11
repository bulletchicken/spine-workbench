import { CircleCheck, House } from 'lucide-react';
import { useRef, useState, type ComponentProps } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { Button } from './Button';
import { Popover } from './Popover';
import './Popover.gallery.css';

const GUARDS = [
  ['The browser blocks it.', 'A content policy allows no address outside the app.'],
  ['The code is not allowed to ask.', 'A lint rule bans every kind of network call.'],
  ['A test watches the wire.', 'The build fails if a single request leaves.'],
] as const;

/** A control and its popover, holding what the on-device badge will show. */
function Badge({ startOpen, side, align }: { startOpen: boolean } & Required<Pick<ComponentProps<typeof Popover>, 'side' | 'align'>>) {
  const [open, setOpen] = useState(startOpen);
  const badge = useRef<HTMLButtonElement>(null);
  return (
    <div className="blocks-gallery-pop-room" data-side={side} data-align={align}>
      <div className="blocks-gallery-anchor">
        <Button
          ref={badge}
          aria-haspopup="dialog"
          aria-expanded={open}
          onPress={() => {
            setOpen(!open);
          }}
        >
          <House />
          On this device only
        </Button>
        <Popover
          open={open}
          onClose={() => {
            setOpen(false);
          }}
          anchor={badge}
          label="On this device only"
          side={side}
          align={align}
        >
          <h3 className="blocks-gallery-pop-title">Nothing leaves this computer.</h3>
          <p className="blocks-gallery-pop-sub">There is no server and no account. Three guards keep it that way.</p>
          <ol className="blocks-gallery-guards">
            {GUARDS.map(([title, line]) => (
              <li key={title}>
                <CircleCheck />
                <div>
                  <b>{title}</b>
                  <span>{line}</span>
                </div>
              </li>
            ))}
          </ol>
          <div className="blocks-gallery-pop-count">
            <b>0</b>
            <span>requests sent this session</span>
          </div>
          <p className="blocks-gallery-pop-fine">
            Films are also stripped on the way in: names, IDs, dates and the file name itself are dropped. Turn the Wi-Fi off and everything still works.
          </p>
        </Popover>
      </div>
    </div>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Popover',
  states: [
    { name: 'Open, below, lined up with its end', show: () => <Badge startOpen side="below" align="end" /> },
    { name: 'Closed: opens below, from its start', show: () => <Badge startOpen={false} side="below" align="start" /> },
    { name: 'Closed: opens above, from its start', show: () => <Badge startOpen={false} side="above" align="start" /> },
  ],
};

export default page;
