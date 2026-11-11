// The words for how sure the engine is, and why. A colour is never the only
// sign: each state has a word, and each reason a short phrase.

import type { ConfidenceReason } from '@/shapes';
import type { Conf } from '@/look-and-feel/data-attributes';

/** The word beside each state: on the key, on a chip, and on the film's mark. */
export const CONF_WORDS: Record<Conf, string> = {
  sure: 'Sure',
  look: 'Worth a look',
  needs: 'Needs you',
  byyou: 'Set by you',
  sacrum: 'Not counted',
};

/** The word on a mark the app filled in, and on a pedicle that could not be seen. */
export const INFERRED_WORD = 'inferred';
export const NOT_VISIBLE_WORD = 'not visible';

/** Said beside the colours until they have been checked against real films. */
export const UNCALIBRATED_NOTE = 'Not yet checked against real films.';

/** Why a vertebra is less sure than its score alone would say. */
export const REASON_PHRASES: Record<ConfidenceReason, string> = {
  'weak-gap': 'The gap above or below is faint',
  'odd-height': 'Its height is unusual next to its neighbours',
  'films-disagree': 'The front and side films place it differently',
  'rib-overlap': 'Ribs cross it',
  'pedicle-hidden': 'A pedicle could not be seen',
  'near-film-edge': 'It is close to the edge of the film',
  'level-uncertain': 'Its level name is in doubt',
  'edited-by-user': 'You changed it',
  'no-evidence': 'The film shows nothing here, so it is placed from its neighbours',
  'tilt-inconsistent': "Its endplate tilts differ from its neighbours'",
  'metal-nearby': 'Metal lies over it',
  'polarity-uncertain': 'The film may be shown bright for dark',
  'lossy-film': 'The film was saved in a way that loses detail',
};
