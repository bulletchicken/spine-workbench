import type { CSSProperties } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { PromisePopover } from './PromisePopover';
import type { RequestsSent } from './requests-sent';

const panel = (requests: RequestsSent) => () => (
  <div className="frame" style={{ position: 'relative', width: 360, height: 260, '--col': '100%' } as CSSProperties}>
    <div className="note glass promise-note open"><PromisePopover requests={requests} /></div>
  </div>
);
const page: GalleryPage = {
  area: 'on-this-device', part: 'PromisePopover', states: [
    { name: 'Nothing sent', show: panel({ sent: 0, blocked: 0 }) },
    { name: 'The browser refused an attempt', show: panel({ sent: 0, blocked: 1 }) },
    { name: 'Several refused', show: panel({ sent: 0, blocked: 3 }) },
    { name: 'The count follows the browser', show: panel({ sent: 2, blocked: 0 }) },
  ],
};
export default page;
