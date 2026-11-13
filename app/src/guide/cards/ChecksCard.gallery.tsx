import { useState } from 'react';
import { Button } from '@/building-blocks/Button';
import type { GalleryPage } from '@/gallery/gallery-page';
import { standingRows } from '@/review/standing-checks';
import { caseAt, findingsAt, STORY_STAGES, type StoryStage } from '@/sample-case/stages';
import type { FilmKind } from '@/shapes';
import '../guide.gallery.css';
import { ChecksCard } from './ChecksCard';
import { standingCheckRows } from './standing-check-rows';

const NAMES: Record<FilmKind, string> = { front: 'Front', side: 'Side', 'bend-left': 'Bending left', 'bend-right': 'Bending right' };
const rowsAt = (stage: StoryStage) => standingCheckRows(standingRows(caseAt(stage).case.marks, findingsAt(stage)), (film) => NAMES[film]);

/** The stages of the sample case's story from the pass on, in order. */
const FROM_THE_PASS = STORY_STAGES.slice(STORY_STAGES.indexOf('pass-result'));

/** The card walked through the story, to see a row flash once as a click settles it. */
function Walked() {
  const [at, setAt] = useState(0);
  const stage = FROM_THE_PASS[at] ?? 'pass-result';
  return (
    <div className="guide-gallery-column">
      <ChecksCard rows={rowsAt(stage)} />
      <span className="gallery-note">
        <Button
          onPress={() => {
            setAt((now) => (now + 1) % FROM_THE_PASS.length);
          }}
        >
          Next: after “{stage}”
        </Button>
      </span>
    </div>
  );
}

const page: GalleryPage = {
  area: 'guide',
  part: 'ChecksCard',
  states: [
    {
      name: 'As the pass leaves the sample case',
      show: () => (
        <div className="guide-gallery-column">
          <ChecksCard rows={rowsAt('pass-result')} />
        </div>
      ),
    },
    {
      name: 'L5 confirmed, one vertebra still missing',
      show: () => (
        <div className="guide-gallery-column">
          <ChecksCard rows={rowsAt('l5-set')} />
        </div>
      ),
    },
    {
      name: 'The standing three, every one passing',
      show: () => (
        <div className="guide-gallery-column">
          <ChecksCard rows={rowsAt('all-clear').slice(0, 3)} />
        </div>
      ),
    },
    {
      name: 'The sample case, all clear: extra rows under the standing three',
      show: () => (
        <div className="guide-gallery-column">
          <ChecksCard rows={rowsAt('all-clear')} />
        </div>
      ),
    },
    { name: 'Walked through the story: a row flashes once when it turns to passing', show: () => <Walked /> },
  ],
};

export default page;
