// What a step tells the guide to say and offer. Every step has a pure
// function from its state to one of these, and its Screen renders
// `<Guide spec={…}>` with this state's cards as children.
//
// One sentence, and at most one of a lit button and a pair of choices:
// when nothing is lit and nothing is asked, there is nothing to do.

import type { ReactNode } from 'react';
import type { IconName } from '@/look-and-feel/icons/icon-name';

/** Words run together, some of them strong: what an eyebrow and a paragraph are made of. */
export type Said = readonly (string | { strong: string })[];

/** One of two equal answers to a question only a person may answer. */
export interface Choice {
  id: string;
  /** The keys that answer it, as `KeyboardEvent.key` names: `['ArrowLeft']`, `['a']`. The first is shown on the card. */
  keys: readonly string[];
  /** "The upper one". */
  title: string;
  /** "Counted up from the sacrum". */
  line: string;
  /** A word in the card's corner, such as "marker" on the side the lead marker reads. */
  tag?: string | undefined;
  /** A small picture of what the choice means, such as `HalfFilmFigure`. */
  figure?: ReactNode;
  choose: () => void;
}

export interface QuietAction {
  label: string;
  press: () => void;
}

export interface LitAction {
  label: string;
  icon?: IconName | undefined;
  press: () => void;
}

export interface GuideSpec {
  /** "Step 2 of 4 · Size & facing · Front film". */
  eyebrow: Said;
  /** One short sentence with a full stop. */
  sentence: string;
  /** One paragraph. */
  body?: Said | undefined;
  /** The next action. At most one of `lit` and `choices`. */
  lit?: LitAction | undefined;
  /** Two equal options, with no default. */
  choices?: readonly [Choice, Choice] | undefined;
  /** The choice whose twin on the film is under the pointer, so its card lights with it. */
  hotChoice?: string | null | undefined;
  /** The pointer or the keyboard's focus is on this choice's card, or on neither: the step lights its twin on the film. */
  onHotChoice?: ((id: string | null) => void) | undefined;
  /** At most two. */
  quiet?: readonly QuietAction[] | undefined;
}
