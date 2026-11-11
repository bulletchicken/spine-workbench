import type { GalleryPage } from '@/gallery/gallery-page';
import type { RowState } from './data-attributes';
import './look-and-feel.gallery.css';
import { StateIcon } from './StateIcon';

const STATES: readonly RowState[] = ['ok', 'wait', 'busy', 'you', 'warn', 'needs', 'bad'];

const ROWS: readonly { state: RowState; title: string; line: string }[] = [
  { state: 'ok', title: 'In order, top to bottom', line: 'No outline overlaps the next' },
  { state: 'wait', title: '17 outlines found', line: 'Counted once L5 is known' },
  { state: 'busy', title: 'Bending left', line: 'Reading…' },
  { state: 'you', title: 'Level names set', line: 'L5 confirmed by you' },
  { state: 'warn', title: 'Level names may be off by one', line: 'Counting from the first rib and from the sacrum disagree' },
  { state: 'bad', title: '16 of 17 vertebrae found', line: 'One missing between T1 and L5' },
];

const page: GalleryPage = {
  area: 'look-and-feel',
  part: 'StateIcon',
  states: [
    {
      name: 'Every state',
      show: () => (
        <div className="look-gallery-set">
          {STATES.map((state) => (
            <span key={state} className="look-gallery-item">
              <StateIcon state={state} />
              <span className="look-gallery-name">{state}</span>
            </span>
          ))}
        </div>
      ),
    },
    {
      name: 'Large, to check the drawing',
      show: () => (
        <div className="look-gallery-set">
          {STATES.map((state) => (
            <span key={state} className="look-gallery-item">
              <StateIcon state={state} className="look-gallery-large" />
              <span className="look-gallery-name">{state}</span>
            </span>
          ))}
        </div>
      ),
    },
    {
      name: 'In the rows of a section',
      show: () => (
        <ul className="look-gallery-rows">
          {ROWS.map(({ state, title, line }) => (
            <li key={state} className="look-gallery-row">
              <StateIcon state={state} />
              <div>
                <b>{title}</b>
                <span>{line}</span>
              </div>
            </li>
          ))}
        </ul>
      ),
    },
    {
      name: 'In a fact chip, 12 px',
      show: () => (
        <>
          <span className="look-gallery-fact" data-state="ok">
            <StateIcon state="ok" />
            0.45 mm/px
          </span>
          <span className="look-gallery-fact" data-state="you">
            <StateIcon state="you" />L on the right
          </span>
          <span className="look-gallery-fact" data-state="needs">
            <StateIcon state="needs" />
            size ?
          </span>
        </>
      ),
    },
  ],
};

export default page;
