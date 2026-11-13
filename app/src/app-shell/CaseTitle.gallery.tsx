import type { GalleryPage } from '@/gallery/gallery-page';
import './app-shell.gallery.css';
import { CaseTitle } from './CaseTitle';

const page: GalleryPage = {
  area: 'app-shell',
  part: 'CaseTitle',
  states: [
    { name: 'A new case with nothing in it', show: () => <CaseTitle name={null} saved="not-saved-yet" /> },
    { name: 'Work that is on no disk', show: () => <CaseTitle name={null} saved="changes-not-saved" /> },
    { name: 'Saved just now: the ring is a solid dot', show: () => <CaseTitle name="spine-case-2026-11-12" saved="saved-just-now" /> },
    { name: 'Opened from a file and not changed', show: () => <CaseTitle name="spine-case-2026-11-12" saved="as-saved" /> },
    {
      name: 'A long name gives way; the state never does',
      show: () => (
        <div className="shell-gallery-brand">
          <CaseTitle name="spine-case-2026-11-12-second-reading-after-the-clinic" saved="changes-not-saved" />
        </div>
      ),
    },
  ],
};

export default page;
