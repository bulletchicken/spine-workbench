// What the outlines layer draws: one vertebra, as plain data. The overlay
// never sees a film's saved marks; the bench turns them into this, and
// says which look each has and whether it is shown yet.

import type { Px } from '@/shapes';
import type { OutlineLook } from '@/look-and-feel/data-attributes';

/** Four corners in the order they go round an outline: upper, upper, lower, lower. */
export type Quad = [Px, Px, Px, Px];

/** One vertebra as the overlay draws it. */
export interface MarkToDraw {
  /** Stable while this vertebra is on screen: its id. */
  key: string;
  /** "T12". */
  name: string;
  /** A body's four corners; on a bending film, the two endplate lines joined into one closed outline. */
  quad: Quad;
  /** Found, or needs a look. */
  look: OutlineLook;
  /** Drawn yet: outlines appear one by one while a search's answer is replayed. */
  shown: boolean;
  /** For a screen reader: "T3", or "T3, needs a look". */
  spoken: string;
}
