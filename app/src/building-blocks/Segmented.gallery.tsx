import { useState, type ReactNode } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { Segmented } from './Segmented';

/** The three steps, as the bar holds them: 3D cannot be pressed until there is a 3D step. */
const STEPS = [
  { id: 'films', label: 'Films' },
  { id: 'find', label: 'Find' },
  { id: 'three', label: '3D', disabled: true },
] as const;

/**
 * The system's .seg is a block grid: it is as narrow as its options only inside a column sized to
 * its content, which the bar gives it. This is that column.
 */
function Column({ children }: { children: ReactNode }) {
  return <div style={{ display: 'grid', gridTemplateColumns: 'auto', justifyContent: 'start' }}>{children}</div>;
}

/** The step control as the bar holds it: press an option and the thumb slides under it. */
function Steps({ start }: { start: string }) {
  const [chosen, setChosen] = useState(start);
  return (
    <Column>
      <Segmented className="steps" label="Steps" options={STEPS} chosen={chosen} onChoose={setChosen} />
    </Column>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Segmented',
  states: [
    { name: 'Films chosen: press Find and the thumb slides; 3D cannot be pressed yet', show: () => <Steps start="films" /> },
    { name: 'Find chosen', show: () => <Steps start="find" /> },
    {
      name: 'A step ahead that cannot be pressed yet',
      show: () => (
        <Column>
          <Segmented className="steps" label="Steps" options={STEPS.map((step) => ({ ...step, disabled: step.id !== 'films' }))} chosen="films" onChoose={() => undefined} />
        </Column>
      ),
    },
    {
      name: 'No option chosen: the thumb is hidden',
      show: () => (
        <Column>
          <Segmented className="steps" label="Steps" options={STEPS} chosen={null} onChoose={() => undefined} />
        </Column>
      ),
    },
  ],
};

export default page;
