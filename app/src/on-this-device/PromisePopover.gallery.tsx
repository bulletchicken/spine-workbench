import type { GalleryPage } from '@/gallery/gallery-page';
import './on-this-device.gallery.css';
import { PromisePopover } from './PromisePopover';
import type { RequestsSent } from './requests-sent';

const panel = (requests: RequestsSent) => () => (
  <div className="promise-gallery-panel">
    <PromisePopover requests={requests} />
  </div>
);

const page: GalleryPage = {
  area: 'on-this-device',
  part: 'PromisePopover',
  states: [
    { name: 'Nothing sent', show: panel({ sent: 0, blocked: 0 }) },
    { name: 'The browser refused an attempt: a line says so', show: panel({ sent: 0, blocked: 1 }) },
    { name: 'Several refused', show: panel({ sent: 0, blocked: 3 }) },
    { name: 'Something was sent: the count is the browser’s, whatever it is', show: panel({ sent: 2, blocked: 0 }) },
  ],
};

export default page;
