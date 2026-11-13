import { useState } from 'react';
import type { ActionRecord } from '@/case-state/action-record';
import type { GalleryPage } from '@/gallery/gallery-page';
import './app-shell.gallery.css';
import type { RailStep } from './rail-steps';
import { TopBar, type TopBarProps } from './TopBar';

const FOUR = ['Films', 'Size & facing', 'Find', 'Adjust'];
const SEVEN = [...FOUR, '3D spine', 'Measure', 'Bones'];

function rail(titles: readonly string[], now: number): RailStep[] {
  return titles.map((title, at) => {
    const number = at + 1;
    return { slug: title, number, title, state: number === now ? 'now' : number < now ? 'done' : 'todo', open: number < now };
  });
}

const SPLIT_T12: ActionRecord = {
  kind: 'replace-marks',
  cause: { by: 'boundary-wand', action: 'split', vertebra: 12 },
  films: ['front', 'side'],
  aimedAt: { film: 'front', vertebra: 12, level: 'T12' },
};
const CONFIRMED_T1: ActionRecord = { kind: 'confirm-vertebra', cause: null, films: ['front', 'side'], aimedAt: { film: 'front', vertebra: 2, level: 'T1' } };

const nothing = () => undefined;

const FIRST_LAUNCH: TopBarProps = {
  caseName: null,
  saved: 'not-saved-yet',
  rail: rail(FOUR, 1),
  onGoToStep: nothing,
  requests: { sent: 0, blocked: 0 },
  promiseOpen: false,
  onPromiseOpenChange: nothing,
  undoIsLive: false,
  nextUndo: null,
  nextRedo: null,
  onUndo: nothing,
  onRedo: nothing,
  saveIsLive: false,
  onSave: nothing,
};

/** A top bar whose badge opens its own popover, so several can stand on one page. */
function Bar({ room = false, ...props }: Partial<TopBarProps> & { room?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="shell-gallery-bar" data-room={room || undefined}>
      <TopBar {...FIRST_LAUNCH} {...props} promiseOpen={open} onPromiseOpenChange={setOpen} />
    </div>
  );
}

const page: GalleryPage = {
  area: 'app-shell',
  part: 'TopBar',
  states: [
    { name: 'First launch: nothing to undo, nothing to save yet', show: () => <Bar /> },
    { name: 'Back on step 2, with films in the case', show: () => <Bar saved="changes-not-saved" rail={rail(FOUR, 2)} /> },
    {
      name: 'Adjust: undo and redo name their action, and Save case is live',
      show: () => <Bar saved="changes-not-saved" rail={rail(FOUR, 4)} undoIsLive nextUndo={SPLIT_T12} nextRedo={CONFIRMED_T1} saveIsLive />,
    },
    {
      name: 'Saved: the ring beside the case is a solid dot',
      show: () => <Bar caseName="spine-case-2026-11-12" saved="saved-just-now" rail={rail(FOUR, 4)} undoIsLive nextUndo={SPLIT_T12} saveIsLive />,
    },
    {
      name: 'A case opened from a file and not changed since',
      show: () => <Bar caseName="spine-case-2026-11-12" saved="as-saved" rail={rail(FOUR, 4)} undoIsLive saveIsLive />,
    },
    {
      name: 'Seven steps: the done ones fold to ticks and the badge gives up its count',
      show: () => <Bar room saved="changes-not-saved" rail={rail(SEVEN, 4)} undoIsLive nextUndo={SPLIT_T12} saveIsLive />,
    },
  ],
};

export default page;
