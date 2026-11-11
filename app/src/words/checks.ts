// The sentence for every finding the case checks can make, and the words of
// the three standing rows. Plain words: what is wrong, and what is still open.

import type { CheckKind } from '@/shapes';

/** One sentence for every check kind, shown beside its finding. */
export const CHECK_SENTENCES: Record<CheckKind, string> = {
  'vertebra-missing': 'A level between T1 and L5 has no vertebra on this film.',
  'levels-out-of-order': 'A level is repeated, or the levels are out of order.',
  'likely-off-by-one': 'Level names may be off by one. Counting from the first rib and from the sacrum disagree.',
  'films-disagree': 'The front and side films do not have the same levels.',
  'non-standard-count': 'This film does not have the usual count of levels.',
  'names-unknown': 'Neither end of the spine could be counted from, so the levels are not named.',
  'l5-unconfirmed': 'Nobody has said which vertebra is L5.',
  'upright-unconfirmed': 'The way this film is turned upright has not been confirmed.',
  'left-right-unconfirmed': "Which side is the patient's left has not been confirmed.",
  'facing-unconfirmed': 'Which way this side film faces has not been confirmed.',
  'scale-unconfirmed': 'This film has no confirmed scale, so lengths cannot be measured on it.',
  'polarity-unconfirmed': 'The brightness of this film has not been confirmed.',
  'privacy-unchecked': 'The border has not been checked for names or IDs.',
  'pedicle-not-visible': 'A pedicle could not be seen, so it is marked not visible.',
  'marks-named-for-other-facing': 'The left and right names were given for a different facing than this film states.',
};

/** The standing rows' titles and lines. */
export const STANDING_WORDS = {
  foundOf: (found: number, of: number): string => `${String(found)} of ${String(of)} vertebrae found`,
  foundOutlines: (found: number): string => `${String(found)} outlines found`,
  foundWaitLine: 'Counted once L5 is known',
  foundMissingLine: (missing: number): string =>
    missing === 1 ? 'One missing between T1 and L5.' : `${String(missing)} missing between T1 and L5.`,
  inOrder: 'In order, top to bottom',
  inOrderOutOfOrder: 'A level is repeated or out of order',
  inOrderFilmsDisagree: 'The front and side films disagree',
  namesSet: 'Level names set',
  namesSetLine: 'L5 confirmed by you',
  namesOffByOne: 'Level names may be off by one',
  namesOffByOneLine: 'Counting from the first rib and from the sacrum disagree',
  namesUnknown: 'Level names are unknown',
  namesUnknownLine: 'Neither end of the spine could be counted from',
  namesNonStandard: 'This count is not the usual one',
  namesNonStandardLine: 'The film does not have the usual count of levels',
  otherFacing: 'Marks are named for another facing',
  /** `film` is the bending film's name as the bench gives it. */
  bendingCount: (film: string): string => `${film} counts a different number of levels from Front`,
} as const;
