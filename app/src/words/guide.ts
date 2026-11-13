// The guide's own words: the eyebrow over the sentence, and the fixed
// headings, counts and rows of its cards. The sentence, the paragraph and
// the buttons under them are each step's, in that step's words file.

import type { Said } from '@/guide/guide-spec';
import type { SlotFace } from '@/look-and-feel/data-attributes';

/** The eyebrow of a step: "Step 2 of 4 · Size & facing · Front film". `about` names what the question is about. */
export function stepEyebrow(number: number, of: number, title: string, about?: string): Said {
  const step = `Step ${String(number)} of ${String(of)} · ${title}`;
  return [about === undefined ? step : `${step} · ${about}`];
}

/** The eyebrow of a review item: "Needs you · 2 of 4", with the count strong. */
export function needsYouEyebrow(at: number, of: number): Said {
  return ['Needs you · ', { strong: `${String(at)} of ${String(of)}` }];
}

export const GUIDE_WORDS = {
  /** The spoken name of a pair of choices. */
  chooseOne: 'Choose one',
} as const;

export const CARD_WORDS = {
  films: {
    heading: 'Films',
    count: (ready: number, of: number): string => `${String(ready)} of ${String(of)}`,
    /** Where a film stands, by the face of its slot. */
    lines: {
      empty: 'Waiting for a file',
      reading: 'Reading…',
      hung: 'Ready. Private details dropped.',
      bad: 'Not an image this app can open',
    } satisfies Record<SlotFace, string>,
  },
  whatHappens: {
    heading: 'What happens to a film',
    rows: [
      { title: 'Kept', line: 'The picture, its pixel size, which view it is' },
      { title: 'Dropped', line: 'Names, IDs, dates and the file name itself' },
      { title: 'Sent anywhere', line: 'Never. There is no server to send it to.' },
    ],
  },
  filmFacts: {
    heading: 'Two facts per film',
    count: (set: number, of: number): string => `${String(set)} of ${String(of)} set`,
  },
  narration: {
    heading: 'What it is doing',
    note: 'Slowed down so you can follow it. Space pauses. Drag the bar under the films to scrub.',
  },
  howSure: {
    heading: 'How sure',
    sameThing: 'Colour, line and mark all say the same thing.',
  },
  checks: {
    heading: 'Checks',
    count: (pass: number, of: number): string => `${String(pass)} of ${String(of)} pass`,
  },
  strictRead: {
    heading: 'Reading strictly',
    count: (read: number, of: number): string => `${String(read)} of ${String(of)}`,
  },
} as const;
