import { useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import './app-shell.gallery.css';
import type { RailStep } from './rail-steps';
import { StepRail } from './StepRail';

const FOUR = ['Films', 'Size & facing', 'Find', 'Adjust'];
const SEVEN = [...FOUR, '3D spine', 'Measure', 'Bones'];

/** A rail of these titles with step `now` on screen: the steps before it done and open, the rest to come. */
function rail(titles: readonly string[], now: number, doneAfter: readonly number[] = []): RailStep[] {
  return titles.map((title, at) => {
    const number = at + 1;
    const done = number < now || doneAfter.includes(number);
    return { slug: title, number, title, state: number === now ? 'now' : done ? 'done' : 'todo', open: number !== now && done };
  });
}

function Rail({ steps }: { steps: RailStep[] }) {
  const [went, setWent] = useState<string | null>(null);
  return (
    <>
      <div className="shell-gallery-rail">
        <StepRail steps={steps} onGo={setWent} />
      </div>
      <span className="gallery-note">{went === null ? 'A done step can be pressed' : `Pressed: ${went}`}</span>
    </>
  );
}

const page: GalleryPage = {
  area: 'app-shell',
  part: 'StepRail',
  states: [
    { name: 'First launch: the first step on screen', show: () => <Rail steps={rail(FOUR, 1)} /> },
    { name: 'The third step on screen, two done', show: () => <Rail steps={rail(FOUR, 3)} /> },
    { name: 'Back on the second step, with the later steps still done', show: () => <Rail steps={rail(FOUR, 2, [3, 4])} /> },
    { name: 'The last of four on screen', show: () => <Rail steps={rail(FOUR, 4)} /> },
    { name: 'Seven steps: the done ones fold down to their ticks', show: () => <Rail steps={rail(SEVEN, 4)} /> },
    { name: 'Seven steps, the sixth on screen', show: () => <Rail steps={rail(SEVEN, 6)} /> },
  ],
};

export default page;
