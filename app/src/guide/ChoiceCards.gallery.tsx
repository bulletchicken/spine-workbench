import { useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { ChoiceCards } from './ChoiceCards';
import './guide.gallery.css';
import type { Choice } from './guide-spec';
import { HalfFilmFigure } from './HalfFilmFigure';

/** A pair of choices that says which was chosen, and which card the pointer or the keyboard is on. */
function Asked({ pair, hot }: { pair: (choose: (answer: string) => () => void) => readonly [Choice, Choice]; hot?: string }) {
  const [answer, setAnswer] = useState('Nothing chosen: there is no default, and Enter answers nothing');
  const [on, setOn] = useState<string | null>(null);
  const choose = (said: string) => () => {
    setAnswer(said);
  };
  return (
    <div className="guide-gallery-part">
      <ChoiceCards choices={pair(choose)} hot={hot ?? null} onHot={setOn} />
      <p className="gallery-note">{answer}</p>
      <p className="gallery-note">{on === null ? 'The pointer and the focus are on neither card' : `On the card “${on}”`}</p>
    </div>
  );
}

const sides = (choose: (answer: string) => () => void): readonly [Choice, Choice] => [
  { id: 'left', keys: ['ArrowLeft'], title: 'This side', line: 'Left of the picture', figure: <HalfFilmFigure half="left" letter="L" />, choose: choose('Chose the left of the picture') },
  {
    id: 'right',
    keys: ['ArrowRight'],
    title: 'This side',
    line: 'Right of the picture',
    tag: 'marker',
    figure: <HalfFilmFigure half="right" letter="L" />,
    choose: choose('Chose the right of the picture'),
  },
];

const page: GalleryPage = {
  area: 'guide',
  part: 'ChoiceCards',
  states: [
    { name: 'Two halves of a film, answered by the arrow keys', show: () => <Asked pair={sides} /> },
    {
      name: 'Two vertebrae, answered by A and B',
      show: () => (
        <Asked
          pair={(choose) => [
            { id: 'a', keys: ['a'], title: 'The upper one', line: 'Counted up from the sacrum', choose: choose('Chose A, the upper one') },
            { id: 'b', keys: ['b'], title: 'The lower one', line: 'Counted down from the first rib', choose: choose('Chose B, the lower one') },
          ]}
        />
      ),
    },
    { name: 'One card lit from its twin on the film', show: () => <Asked pair={sides} hot="right" /> },
  ],
};

export default page;
