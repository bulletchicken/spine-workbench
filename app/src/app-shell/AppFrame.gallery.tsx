import type { CSSProperties } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import type { StepContract } from '@/steps/step-contract';
import { currentStep } from '@/view-state/current-step';
import { useShowInDock } from '@/view-state/dock-now';
import { AppFrame } from './AppFrame';

function Films() {
  useShowInDock({
    scene: 'ready', sentence: 'All four films are in.',
    helper: { kind: 'words', words: ['Each one is in its place.'] },
    primary: { label: 'Find the spine', press: () => { currentStep.getState().go('find'); } },
  });
  return null;
}
function Find() {
  useShowInDock({ scene: 'found', sentence: '17 vertebrae found.', helper: { kind: 'words', words: ['All of them look right.'] } });
  return null;
}
const steps: readonly StepContract[] = [
  { number: 1, slug: 'films', title: 'Films', Screen: Films, isReady: () => true, canLeave: () => true },
  { number: 2, slug: 'find', title: 'Find', Screen: Find, isReady: () => true, canLeave: () => true },
];
const page: GalleryPage = {
  area: 'app-shell', part: 'AppFrame', states: [{
    name: 'The one stage frame, with an empty stage',
    show: () => (
      <div style={{ position: 'relative', width: '100%', height: 760, overflow: 'hidden', transform: 'translate(0)', '--frame-w': 'calc(100% - 32px)', '--frame-h': '100%' } as CSSProperties}>
        <AppFrame steps={steps} parts={{ Bench: null, CaseSheet: null, OpeningCase: null }} />
      </div>
    ),
  }],
};
export default page;
