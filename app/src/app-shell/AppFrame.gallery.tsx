import type { GalleryPage } from '@/gallery/gallery-page';
import { FilmsCard } from '@/guide/cards/FilmsCard';
import { WhatHappensCard } from '@/guide/cards/WhatHappensCard';
import { Guide } from '@/guide/Guide';
import type { StepContract } from '@/steps/step-contract';
import { currentStep } from '@/view-state/current-step';
import { stepEyebrow } from '@/words/guide';
import './app-shell.gallery.css';
import { AppFrame } from './AppFrame';

// The whole frame on four fixture steps, with an empty bench unless the
// app has one. The first step shows the design's first-launch guide, to
// put beside the mock's first screen; every step's lit button goes on to
// the next, so the rail can be walked forward and back. A fixture step is
// done once the walk has gone past it.

const TITLES = ['Films', 'Size & facing', 'Find', 'Adjust'];
const SLUGS = ['films', 'size-and-facing', 'find', 'adjust'];
const FILMS = ['Front', 'Side', 'Bending left', 'Bending right'];

/** How far the walk has gone: the place of the furthest step opened. */
let reached = 0;

function fixtureStep(at: number): StepContract {
  const next = SLUGS[at + 1];
  const title = TITLES[at] ?? '';
  function Screen() {
    const lit =
      next === undefined
        ? undefined
        : {
            label: at === 0 ? 'Choose films' : 'Go on',
            press: () => {
              reached = Math.max(reached, at + 1);
              currentStep.getState().go(next);
            },
          };
    return at === 0 ? (
      <Guide
        spec={{
          eyebrow: stepEyebrow(1, 4, title),
          sentence: 'Hang your four films.',
          body: ['Front, side, and one bending each way. Drop the files anywhere on this window, or choose them.'],
          lit,
          quiet: [
            { label: 'Open a saved case', press: () => undefined },
            { label: 'Try a made-up spine', press: () => undefined },
          ],
        }}
      >
        <FilmsCard films={FILMS.map((name) => ({ id: name, name, face: 'empty' }))} />
        <WhatHappensCard />
      </Guide>
    ) : (
      <Guide spec={{ eyebrow: stepEyebrow(at + 1, 4, title), sentence: `This is the ${title} step.`, body: ['A fixture, to walk the rail with.'], lit }} />
    );
  }
  return { number: at + 1, slug: SLUGS[at] ?? '', title, Screen, isReady: () => at <= reached, canLeave: () => at < reached };
}

const STEPS = SLUGS.map((_, at) => fixtureStep(at));

const page: GalleryPage = {
  area: 'app-shell',
  part: 'AppFrame',
  states: [
    {
      name: 'The frame on four steps: top bar, guide, bench, status line',
      show: () => (
        <div className="shell-gallery-window">
          <AppFrame steps={STEPS} />
        </div>
      ),
    },
  ],
};

export default page;
