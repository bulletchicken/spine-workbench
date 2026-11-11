import type { ReactNode } from 'react';
import './ConfidenceGlyph.css';
import type { Conf } from './data-attributes';

// The five glyphs, one per confidence state. Each is a different shape, so
// the states are told apart with no colour at all: a filled dot, a half
// dot, a hollow ring, a tick and a short broken dash. All are drawn 12
// units across and centred on the origin, to the same optical size.

const SHAPES: Record<Conf, ReactNode> = {
  // Sure: a filled dot.
  sure: <circle className="confidence-glyph-solid" r="4.2" />,
  // Worth a look: a ring with its left half filled.
  look: (
    <>
      <circle className="confidence-glyph-line" r="3.7" strokeWidth="1.5" />
      <path className="confidence-glyph-solid" d="M0-3.7A3.7 3.7 0 0 0 0 3.7Z" />
    </>
  ),
  // Needs you: a hollow ring, the heaviest line of the five.
  needs: <circle className="confidence-glyph-line" r="3.45" strokeWidth="2.1" />,
  // Set by you: a tick.
  byyou: <path className="confidence-glyph-line" strokeWidth="1.9" d="M-4 .5l2.7 2.7L4-3" />,
  // Not counted, or inferred: a short broken dash.
  sacrum: <path className="confidence-glyph-line" strokeWidth="1.7" d="M-4.2 0h2.9M1.3 0h2.9" />,
};

/**
 * What lies under each glyph as its casing: its own outline, a little
 * larger. The two glyphs with a hollow are cased with a whole disc, so the
 * hollow is evenly dark and no bright bone shows through the ring.
 */
const CASINGS: Record<Conf, ReactNode> = {
  sure: <circle className="confidence-glyph-casing" r="4.2" strokeWidth="2.4" />,
  look: <circle className="confidence-glyph-casing-disc" r="5.65" />,
  needs: <circle className="confidence-glyph-casing-disc" r="5.7" />,
  byyou: <path className="confidence-glyph-casing" strokeWidth="4.3" d="M-4 .5l2.7 2.7L4-3" />,
  sacrum: <path className="confidence-glyph-casing" strokeWidth="4.1" d="M-4.2 0h2.9M1.3 0h2.9" />,
};

interface ConfidenceGlyphShapesProps {
  conf: Conf;
  /** Lay the dark casing under the glyph. For a glyph drawn on a film. */
  cased?: boolean;
}

/**
 * A glyph's shapes alone, for drawing inside another SVG: 12 units across,
 * centred on the origin. The group around them places and sizes them, and
 * an ancestor's `data-conf` colours them.
 */
export function ConfidenceGlyphShapes({ conf, cased = false }: ConfidenceGlyphShapesProps) {
  return (
    <>
      {cased && CASINGS[conf]}
      {SHAPES[conf]}
    </>
  );
}

interface ConfidenceGlyphProps {
  conf: Conf;
  className?: string | undefined;
}

/**
 * One confidence glyph as a 12 px picture, coloured by its own `data-conf`.
 * Decoration only: the word for the state always stands beside it.
 */
export function ConfidenceGlyph({ conf, className }: ConfidenceGlyphProps) {
  return (
    <svg
      className={className === undefined ? 'confidence-glyph' : `confidence-glyph ${className}`}
      data-conf={conf}
      viewBox="-6 -6 12 12"
      aria-hidden="true"
      focusable="false"
    >
      <ConfidenceGlyphShapes conf={conf} />
    </svg>
  );
}
