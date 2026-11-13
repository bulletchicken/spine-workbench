// What each card counts and shows from the plain props it is given, and
// the one thing a card does over time: a check flashes once when it turns
// from open to passing.

import { describe, expect, it } from 'vitest';
import { endAnimation, find, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import type { CardRow } from './CardRows';
import { ChecksCard } from './ChecksCard';
import { FilmFactsCard } from './FilmFactsCard';
import { FilmsCard } from './FilmsCard';
import { HowSureCard } from './HowSureCard';
import { NarrationCard } from './NarrationCard';
import { StrictReadCard } from './StrictReadCard';
import { WhatHappensCard } from './WhatHappensCard';

const texts = (selector: string) => [...document.querySelectorAll(selector)].map((element) => element.textContent);
const states = () => [...document.querySelectorAll('.card-row')].map((row) => row.getAttribute('data-state'));

describe('the films card', () => {
  it('counts the films that are up, and says where each stands', () => {
    putOnThePage(
      <FilmsCard
        films={[
          { id: 'front', name: 'Front', face: 'hung' },
          { id: 'side', name: 'Side', face: 'reading' },
          { id: 'bend-left', name: 'Bending left', face: 'empty' },
          { id: 'bend-right', name: 'Bending right', face: 'bad', line: 'This file ends too soon' },
        ]}
      />,
    );
    expect(find('.guide-card-head').textContent).toBe('Films1 of 4');
    expect(states()).toEqual(['ok', 'busy', 'wait', 'bad']);
    expect(texts('.card-row-line')).toEqual(['Ready. Private details dropped.', 'Reading…', 'Waiting for a file', 'This file ends too soon']);
  });
});

describe('the film facts card', () => {
  it('counts a film as set when its facts were found or a person set them', () => {
    putOnThePage(
      <FilmFactsCard
        films={[
          { id: 'front', name: 'Front', state: 'you', facts: ['0.45 mm per pixel, from the file', 'Confirmed by you'] },
          { id: 'side', name: 'Side', state: 'ok', facts: ['0.45 mm per pixel, from the file', 'Faces right'] },
          { id: 'bend-right', name: 'Bending right', state: 'warn', facts: ['No size in the file', 'Upright'] },
        ]}
      />,
    );
    expect(find('.guide-card-count').textContent).toBe('2 of 3 set');
    expect(texts('.card-row:last-child .card-row-line')).toEqual(['No size in the file', 'Upright']);
  });
});

describe('what happens to a film', () => {
  it('says what is kept, what is dropped, and that it is sent nowhere', () => {
    putOnThePage(<WhatHappensCard />);
    expect(texts('.card-row-title')).toEqual(['Kept', 'Dropped', 'Sent anywhere']);
    expect(texts('.card-row-line')[2]).toBe('Never. There is no server to send it to.');
  });
});

describe('the narration card', () => {
  const LINES = ['One', 'Two', 'Three'].map((title) => ({ title, line: `More about ${title}` }));
  const marks = () => [...document.querySelectorAll('.narration-line')].map((line) => [line.hasAttribute('data-done'), line.hasAttribute('data-on')]);

  it('opens the live line and ticks the ones before it', () => {
    putOnThePage(<NarrationCard lines={LINES} live={1} finished={1} />);
    expect(marks()).toEqual([
      [true, false],
      [false, true],
      [false, false],
    ]);
    expect(texts('.narration-number')).toEqual(['', '2', '3']);
    // Only the open line's explanation is spoken.
    expect([...document.querySelectorAll('.narration-more')].map((more) => more.getAttribute('aria-hidden'))).toEqual(['true', 'false', 'true']);
    expect(find('[aria-current="step"]').textContent).toContain('Two');
  });

  it('ticks every line once the pass is over, with none open', () => {
    putOnThePage(<NarrationCard lines={LINES} live={null} finished={3} />);
    expect(marks()).toEqual([
      [true, false],
      [true, false],
      [true, false],
    ]);
  });
});

describe('the how-sure card', () => {
  const COUNTS = { sure: 13, look: 2, needs: 2, byyou: 0 };

  it('is the key and the tally in one: four chips, each with its state, word and count', () => {
    putOnThePage(<HowSureCard counts={COUNTS} calibrated />);
    expect([...document.querySelectorAll('.how-sure-chip')].map((chip) => chip.getAttribute('data-conf'))).toEqual(['sure', 'look', 'needs', 'byyou']);
    expect(texts('.how-sure-word')).toEqual(['Sure', 'Worth a look', 'Needs you', 'Set by you']);
    expect(texts('.how-sure-count')).toEqual(['13', '2', '2', '0']);
    expect(document.querySelector('.guide-card-note')).toBeNull();
  });

  it('says so until confidence has been checked against real films', () => {
    putOnThePage(<HowSureCard counts={COUNTS} calibrated={false} />);
    expect(find('.guide-card-note').textContent).toBe('Not yet checked against real films.');
  });

  it('can close with the key’s note', () => {
    putOnThePage(<HowSureCard counts={COUNTS} calibrated withKey />);
    expect(find('.guide-card-note').textContent).toBe('Colour, line and mark all say the same thing.');
  });
});

describe('the strict-read card', () => {
  it('ticks only the lines it is told were checked', () => {
    putOnThePage(
      <StrictReadCard
        lines={[
          { line: 'It is a spine-case file, version 1', state: 'ok' },
          { line: 'Four films under their fixed names', state: 'bad' },
          { line: 'The data file matches the format exactly', state: 'wait' },
        ]}
      />,
    );
    expect(find('.guide-card-count').textContent).toBe('1 of 3');
    expect(states()).toEqual(['ok', 'bad', 'wait']);
  });
});

describe('the checks card', () => {
  const OPEN: CardRow[] = [
    { id: 'found', state: 'bad', title: '16 of 17 vertebrae found', lines: ['One missing between T1 and L5.'] },
    { id: 'in-order', state: 'ok', title: 'In order, top to bottom' },
    { id: 'names', state: 'warn', title: 'Level names may be off by one' },
  ];
  const NAMED: CardRow[] = [...OPEN.slice(0, 2), { id: 'names', state: 'you', title: 'Level names set', lines: ['L5 confirmed by you'] }];
  const flashing = () => [...document.querySelectorAll('.card-row')].map((row) => row.hasAttribute('data-flash'));

  it('counts the checks that pass, a person’s among them', () => {
    const page = putOnThePage(<ChecksCard rows={OPEN} />);
    expect(find('.guide-card-count').textContent).toBe('1 of 3 pass');
    page.show(<ChecksCard rows={NAMED} />);
    expect(find('.guide-card-count').textContent).toBe('2 of 3 pass');
  });

  it('counts an extra row with the standing three', () => {
    putOnThePage(<ChecksCard rows={[...NAMED, { id: 'other-facing', state: 'warn', title: 'Marks are named for another facing' }]} />);
    expect(find('.guide-card-count').textContent).toBe('2 of 4 pass');
  });

  it('flashes a row once when it turns from open to passing, and no other row', () => {
    const page = putOnThePage(<ChecksCard rows={OPEN} />);
    // Nothing flashes on arrival, not even the row that already passes.
    expect(flashing()).toEqual([false, false, false]);

    page.show(<ChecksCard rows={NAMED} />);
    expect(flashing()).toEqual([false, false, true]);

    // The flash runs once: when it ends the mark comes off, and a later render that changes nothing does not bring it back.
    const row = document.querySelectorAll('.card-row')[2];
    if (row === undefined) throw new Error('the names row is not on the page');
    endAnimation(row);
    expect(flashing()).toEqual([false, false, false]);
    page.show(<ChecksCard rows={[...NAMED]} />);
    expect(flashing()).toEqual([false, false, false]);
  });

  it('does not flash a row that turns back to open', () => {
    const page = putOnThePage(<ChecksCard rows={NAMED} />);
    page.show(<ChecksCard rows={OPEN} />);
    expect(flashing()).toEqual([false, false, false]);
  });
});
