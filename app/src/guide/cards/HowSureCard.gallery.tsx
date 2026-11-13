import type { GalleryPage } from '@/gallery/gallery-page';
import { tally } from '@/review/tally';
import { caseAt } from '@/sample-case/stages';
import '../guide.gallery.css';
import { HowSureCard } from './HowSureCard';

const page: GalleryPage = {
  area: 'guide',
  part: 'HowSureCard',
  states: [
    {
      name: 'While the pass plays: nothing counted yet',
      show: () => (
        <div className="guide-gallery-column">
          <HowSureCard counts={{ sure: 0, look: 0, needs: 0, byyou: 0 }} calibrated={false} />
        </div>
      ),
    },
    {
      name: 'The sample case as the pass leaves it',
      show: () => (
        <div className="guide-gallery-column">
          <HowSureCard counts={tally(caseAt('pass-result').case.marks)} calibrated={false} />
        </div>
      ),
    },
    {
      name: 'All clear, with the key’s note',
      show: () => (
        <div className="guide-gallery-column">
          <HowSureCard counts={tally(caseAt('all-clear').case.marks)} calibrated={false} withKey />
        </div>
      ),
    },
    {
      name: 'Once confidence has been checked against real films',
      show: () => (
        <div className="guide-gallery-column">
          <HowSureCard counts={tally(caseAt('all-clear').case.marks)} calibrated withKey />
        </div>
      ),
    },
  ],
};

export default page;
