import { useState, type ReactNode } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { standingRows } from '@/review/standing-checks';
import { tally } from '@/review/tally';
import { caseAt, findingsAt, type StoryStage } from '@/sample-case/stages';
import type { FilmKind } from '@/shapes';
import { needsYouEyebrow, stepEyebrow } from '@/words/guide';
import { ChecksCard } from './cards/ChecksCard';
import { FilmFactsCard, type FilmFactsCardFilm } from './cards/FilmFactsCard';
import { FilmsCard, type FilmsCardFilm } from './cards/FilmsCard';
import { HowSureCard } from './cards/HowSureCard';
import { NarrationCard } from './cards/NarrationCard';
import { standingCheckRows } from './cards/standing-check-rows';
import { StrictReadCard } from './cards/StrictReadCard';
import { WhatHappensCard } from './cards/WhatHappensCard';
import { Guide } from './Guide';
import './guide.gallery.css';
import type { GuideSpec } from './guide-spec';
import { HalfFilmFigure } from './HalfFilmFigure';

// Every state of the design's guide, first launch through saved, reopened
// and refused, each with its cards. The sentences are the design's; in the
// app each step keeps its own in its words file. The cards' numbers are
// worked out from the sample case at the matching moment of its story.

const NAMES: Record<FilmKind, string> = { front: 'Front', side: 'Side', 'bend-left': 'Bending left', 'bend-right': 'Bending right' };
const KINDS = Object.keys(NAMES) as FilmKind[];
const filmName = (film: FilmKind): string => NAMES[film];

const films = (faces: FilmsCardFilm['face'][]): FilmsCardFilm[] => KINDS.map((kind, at) => ({ id: kind, name: NAMES[kind], face: faces[at] ?? 'empty' }));

const SIZE = '0.45 mm per pixel, from the file';
function facts(side: 'open' | 'set', size: 'missing' | 'set'): FilmFactsCardFilm[] {
  return [
    {
      id: 'front',
      name: NAMES.front,
      state: side === 'open' ? 'warn' : 'you',
      facts: [SIZE, side === 'open' ? 'Left and right: waiting for you' : 'Patient’s left is on the right of the picture, confirmed by you'],
    },
    { id: 'side', name: NAMES.side, state: 'ok', facts: [SIZE, 'Faces right, read from the bulge of the upper back'] },
    { id: 'bend-left', name: NAMES['bend-left'], state: 'ok', facts: [SIZE, 'It came in upside-down and was turned upright'] },
    {
      id: 'bend-right',
      name: NAMES['bend-right'],
      state: size === 'missing' ? 'warn' : 'you',
      facts: [size === 'missing' ? 'No size in the file' : '0.45 mm per pixel, from your two clicks on a 25 mm ball', 'Upright'],
    },
  ];
}

const NARRATION = [
  { title: 'Looking for bone', line: 'Edges, ridges and pedicle rings are picked out on both films. Ribs are edges too.' },
  { title: 'Tracing the centre of the spine', line: 'The cheapest path through that evidence, neck to pelvis.' },
  { title: 'Straightening the spine', line: 'Unrolled into a strip, bone and disc gaps read like a barcode.' },
  { title: 'Searching for all 17 at once', line: 'One sweep weighs every way of placing them. Both films vote. Dashed outlines are guesses, not answers.' },
  { title: 'Locking in and naming', line: 'The best chain is only known at the end, so it is read back from the pelvis up.' },
  { title: 'Rating how sure it is', line: 'Blue is sure. Yellow is worth a look. Coral needs you.' },
];

const NO_COUNTS = { sure: 0, look: 0, needs: 0, byyou: 0 };

/** How many of the sample case's vertebrae a person had set by the time it is all clear. */
const SET_BY_YOU = tally(caseAt('all-clear').case.marks).byyou;
const IN_WORDS = ['No', 'One', 'Two', 'Three', 'Four', 'Five', 'Six', 'Seven', 'Eight', 'Nine'];

/**
 * The checks and the tally as they stand at one moment of the sample
 * case's story. The standing three only, as the design draws them: the
 * rows a bending film can add are on the checks card's own page.
 */
function ResultCards({ stage, withKey = false }: { stage: StoryStage; withKey?: boolean }) {
  const marks = caseAt(stage).case.marks;
  return (
    <>
      <ChecksCard rows={standingCheckRows(standingRows(marks, findingsAt(stage)), filmName).slice(0, 3)} />
      <HowSureCard counts={tally(marks)} calibrated={false} withKey={withKey} />
    </>
  );
}

type Say = (what: string) => () => void;

/** A column as wide as the guide's own, with a line under it saying what was last pressed or chosen. */
function GuideColumnPlate({ children }: { children: (say: Say) => ReactNode }) {
  const [said, setSaid] = useState('Nothing pressed yet');
  const say: Say = (what) => () => {
    setSaid(what);
  };
  return (
    <div className="guide-gallery-column">
      {children(say)}
      <span className="gallery-note">{said}</span>
    </div>
  );
}

function state(name: string, spec: (say: Say) => GuideSpec, cards?: ReactNode): GalleryPage['states'][number] {
  return {
    name,
    show: () => <GuideColumnPlate>{(say) => <Guide spec={spec(say)}>{cards}</Guide>}</GuideColumnPlate>,
  };
}

const page: GalleryPage = {
  area: 'guide',
  part: 'Guide',
  states: [
    state(
      'First launch',
      (say) => ({
        eyebrow: stepEyebrow(1, 4, 'Films'),
        sentence: 'Hang your four films.',
        body: ['Front, side, and one bending each way. Drop the files anywhere on this window, or choose them.'],
        lit: { label: 'Choose films', press: say('Choose films') },
        quiet: [
          { label: 'Open a saved case', press: say('Open a saved case') },
          { label: 'Try a made-up spine', press: say('Try a made-up spine') },
        ],
      }),
      <>
        <FilmsCard films={films(['empty', 'empty', 'empty', 'empty'])} />
        <WhatHappensCard />
      </>,
    ),
    state(
      'Films being read',
      () => ({
        eyebrow: stepEyebrow(1, 4, 'Films'),
        sentence: 'Reading your films…',
        body: ['Each one is read here, on this computer. Names, IDs and dates are dropped on the way in.'],
      }),
      <>
        <FilmsCard films={films(['hung', 'reading', 'reading', 'reading'])} />
        <WhatHappensCard />
      </>,
    ),
    state(
      'A film can’t be read',
      (say) => ({
        eyebrow: stepEyebrow(1, 4, 'Films'),
        sentence: 'One film couldn’t be read.',
        body: [{ strong: 'Bending right' }, ' isn’t a PNG, a JPEG or a DICOM image. The other three are up and nothing was lost.'],
        lit: { label: 'Choose another file', press: say('Choose another file') },
        quiet: [{ label: 'Start over', press: say('Start over') }],
      }),
      <>
        <FilmsCard films={films(['hung', 'hung', 'hung', 'bad'])} />
        <WhatHappensCard />
      </>,
    ),
    state(
      'Four films up',
      (say) => ({
        eyebrow: stepEyebrow(1, 4, 'Films'),
        sentence: 'Four films are up.',
        body: ['Next, two quick facts about each one: how big a pixel is, and which way it faces.'],
        lit: { label: 'Check size and facing', press: say('Check size and facing') },
        quiet: [{ label: 'Start over', press: say('Start over') }],
      }),
      <>
        <FilmsCard films={films(['hung', 'hung', 'hung', 'hung'])} />
        <WhatHappensCard />
      </>,
    ),
    state(
      'Left or right',
      (say) => ({
        eyebrow: stepEyebrow(2, 4, 'Size & facing', 'Front film'),
        sentence: 'Which side is the patient’s left?',
        body: [
          'The lead marker on this film reads ',
          { strong: 'L' },
          ', on the right of the picture. A mirrored spine still looks normal, so a person always confirms this. Click that side of the film, or choose here.',
        ],
        choices: [
          { id: 'left', keys: ['ArrowLeft'], title: 'This side', line: 'Left of the picture', figure: <HalfFilmFigure half="left" letter="L" />, choose: say('Chose the left of the picture') },
          {
            id: 'right',
            keys: ['ArrowRight'],
            title: 'This side',
            line: 'Right of the picture',
            tag: 'marker',
            figure: <HalfFilmFigure half="right" letter="L" />,
            choose: say('Chose the right of the picture'),
          },
        ],
      }),
      <FilmFactsCard films={facts('open', 'missing')} />,
    ),
    state(
      'No size: two-click ruler',
      (say) => ({
        eyebrow: stepEyebrow(2, 4, 'Size & facing', 'Bending right'),
        sentence: 'Bending right has no size.',
        body: ['Its file doesn’t say how big a pixel is. Click the two ends of anything whose length you know. The marker ball on this film is 25 mm across.'],
        lit: { label: 'Measure with two clicks', icon: 'ruler', press: say('Measure with two clicks') },
        quiet: [{ label: 'Same as the front film', press: say('Same as the front film') }],
      }),
      <FilmFactsCard films={facts('set', 'missing')} />,
    ),
    state(
      'Ruler: placing the ends',
      (say) => ({
        eyebrow: stepEyebrow(2, 4, 'Size & facing', 'Bending right'),
        sentence: 'Now the other end.',
        body: ['Click the two ends of the marker ball on the bending-right film.'],
        quiet: [
          { label: 'Cancel', press: say('Cancel') },
          { label: 'Same as the front film', press: say('Same as the front film') },
        ],
      }),
      <FilmFactsCard films={facts('set', 'missing')} />,
    ),
    state(
      'Ruler: measured',
      (say) => ({
        eyebrow: stepEyebrow(2, 4, 'Size & facing', 'Bending right'),
        sentence: 'How long is that?',
        body: ['The line is ', { strong: '56 pixels' }, ' long. Type its real length beside it. At 25 mm, a pixel on this film is 0.45 mm.'],
        lit: { label: 'Set the size', press: say('Set the size') },
        quiet: [{ label: 'Measure again', press: say('Measure again') }],
      }),
      <FilmFactsCard films={facts('set', 'missing')} />,
    ),
    state(
      'Films all set',
      (say) => ({
        eyebrow: stepEyebrow(2, 4, 'Size & facing'),
        sentence: 'All four films are set.',
        body: ['Every film has a size and a facing. Nothing was resized and nothing was mirrored: these are facts stored beside the film.'],
        lit: { label: 'Find the spine', press: say('Find the spine') },
        quiet: [{ label: 'Change left and right', press: say('Change left and right') }],
      }),
      <FilmFactsCard films={facts('set', 'set')} />,
    ),
    state(
      'Automatic pass',
      (say) => ({
        eyebrow: stepEyebrow(3, 4, 'Find'),
        sentence: 'Watch it find the spine.',
        body: ['About twelve seconds, slowed down so you can follow it. Nothing is final until the last step, so an early outline may still move.'],
        quiet: [{ label: 'Skip to the result', press: say('Skip to the result') }],
      }),
      <>
        <NarrationCard lines={NARRATION} live={3} finished={3} />
        <HowSureCard counts={NO_COUNTS} calibrated={false} />
      </>,
    ),
    state(
      'Pass result',
      (say) => ({
        eyebrow: stepEyebrow(3, 4, 'Find'),
        sentence: 'Found 17. Four need a look.',
        body: ['Blue outlines are sure. Yellow and coral mark where the films were hard to read. We’ll go through those one at a time.'],
        lit: { label: 'Look at the first one', press: say('Look at the first one') },
        quiet: [{ label: 'Watch again', press: say('Watch again') }],
      }),
      <ResultCards stage="pass-result" />,
    ),
    state(
      'Review: which is L5',
      (say) => ({
        eyebrow: needsYouEyebrow(1, 4),
        sentence: 'Which vertebra is L5?',
        body: ['Counting down from the first rib and counting up from the sacrum land one apart. L5 is the last vertebra above the sacrum. Pick it here or on the film.'],
        choices: [
          { id: 'a', keys: ['a'], title: 'The upper one', line: 'Counted up from the sacrum', choose: say('Chose A, the upper one') },
          { id: 'b', keys: ['b'], title: 'The lower one', line: 'Counted down from the first rib', choose: say('Chose B, the lower one') },
        ],
        quiet: [{ label: 'Skip for now', press: say('Skip for now') }],
      }),
      <ResultCards stage="pass-result" withKey />,
    ),
    state(
      'Review: split',
      (say) => ({
        eyebrow: needsYouEyebrow(2, 4),
        sentence: 'One vertebra is missing.',
        body: [
          'There are 16 between the first rib and the sacrum, and ',
          { strong: 'T12' },
          ' is about twice as tall as its neighbours. It is probably two. The ring on the film is the disc gap inside it.',
        ],
        lit: { label: 'Split T12 at the gap', icon: 'boundary-wand', press: say('Split T12 at the gap') },
        quiet: [
          { label: 'It really is one', press: say('It really is one') },
          { label: 'Skip for now', press: say('Skip for now') },
        ],
      }),
      <ResultCards stage="l5-set" withKey />,
    ),
    state(
      'Review: fix T3',
      (say) => ({
        eyebrow: needsYouEyebrow(3, 4),
        sentence: 'T3 is hard to see.',
        body: ['The shoulder covers it, and the outline stops short of the bone on one side. The ring on the film is the bone it missed. You can also drag a corner.'],
        lit: { label: 'Add the missed bone', icon: 'region-wand', press: say('Add the missed bone') },
        quiet: [
          { label: 'It looks right', press: say('It looks right') },
          { label: 'Skip for now', press: say('Skip for now') },
        ],
      }),
      <ResultCards stage="after-split" withKey />,
    ),
    state(
      'Review: confirm T1',
      (say) => ({
        eyebrow: needsYouEyebrow(4, 4),
        sentence: 'Is T1 right?',
        body: [
          'It is partly hidden by the shoulder, so it is worth a look. One pedicle could not be seen and is marked ',
          { strong: 'not visible' },
          ', not guessed. Drag a corner if it is off.',
        ],
        lit: { label: 'Looks right', icon: 'check', press: say('Looks right') },
        quiet: [{ label: 'Skip for now', press: say('Skip for now') }],
      }),
      <ResultCards stage="t3-fixed" withKey />,
    ),
    state(
      'Free adjusting, with things left',
      (say) => ({
        eyebrow: stepEyebrow(4, 4, 'Adjust'),
        sentence: 'Two things still need a look.',
        body: ['Adjust anything you like. The tools are on the left, and every change can be undone.'],
        lit: { label: 'Go to the next one', press: say('Go to the next one') },
        quiet: [{ label: 'Save as it is', press: say('Save as it is') }],
      }),
      <ResultCards stage="after-split" withKey />,
    ),
    state(
      'Things skipped',
      (say) => ({
        eyebrow: stepEyebrow(4, 4, 'Adjust'),
        sentence: 'One thing skipped.',
        body: ['You can save now and finish later. The case file remembers what is still open.'],
        lit: { label: 'Save the case', icon: 'case-file', press: say('Save the case') },
        quiet: [{ label: 'Go back to what was skipped', press: say('Go back to what was skipped') }],
      }),
      <ResultCards stage="t3-fixed" withKey />,
    ),
    state(
      'All clear',
      (say) => ({
        eyebrow: stepEyebrow(4, 4, 'Adjust'),
        sentence: 'Everything checks out.',
        body: [`17 vertebrae, in order, named T1 to L5. ${IN_WORDS[SET_BY_YOU] ?? String(SET_BY_YOU)} were set by you.`],
        lit: { label: 'Save the case', icon: 'case-file', press: say('Save the case') },
      }),
      <ResultCards stage="all-clear" withKey />,
    ),
    state(
      'Saved',
      (say) => ({
        eyebrow: ['Saved'],
        sentence: 'Saved to this computer.',
        body: ['Nothing was uploaded. Drop the case file on this window any time to pick up where you left off.'],
        quiet: [
          { label: 'Start a new case', press: say('Start a new case') },
          { label: 'Save again', press: say('Save again') },
        ],
      }),
      <ResultCards stage="all-clear" withKey />,
    ),
    state(
      'Reopened',
      (say) => ({
        eyebrow: ['Saved'],
        sentence: 'Everything is as you left it.',
        body: [`Read strictly from the case file: 4 films, 17 vertebrae, ${String(SET_BY_YOU)} of them set by you.`],
        quiet: [
          { label: 'Start a new case', press: say('Start a new case') },
          { label: 'Save again', press: say('Save again') },
        ],
      }),
      <ResultCards stage="all-clear" withKey />,
    ),
    state(
      'Reopen a case',
      () => ({
        eyebrow: ['Open a saved case'],
        sentence: 'Opening the case…',
        body: ['The file is read strictly, line by line, before anything is shown.'],
      }),
      <StrictReadCard
        lines={[
          { line: 'It is a spine-case file, version 1', state: 'ok' },
          { line: 'Four films under their fixed names', state: 'ok' },
          { line: 'The data file matches the format exactly', state: 'busy' },
          { line: '17 vertebrae, pins and L5 as saved', state: 'wait' },
        ]}
      />,
    ),
    state('Case won’t open', (say) => ({
      eyebrow: ['Open a saved case'],
      sentence: 'This case file can’t be opened.',
      body: ['Its data file does not match the format this app reads (', { strong: 'spine-case, version 1' }, '). Nothing was changed and nothing was guessed.'],
      lit: { label: 'Choose another file', press: say('Choose another file') },
      quiet: [{ label: 'Back to an empty bench', press: say('Back to an empty bench') }],
    })),
  ],
};

export default page;
