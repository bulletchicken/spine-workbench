import { useState, type ReactNode } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { ConfidenceGlyph } from '@/look-and-feel/ConfidenceGlyph';
import './choreography.gallery.css';
import { placesFromEdit, placeStyle } from './stagger';

/** Shows a one-shot animation, with a button that plays it again by mounting its subject afresh. */
function Replayed({ children }: { children: ReactNode }) {
  const [plays, setPlays] = useState(0);
  return (
    <div className="motion-gallery-stack">
      <div key={plays}>{children}</div>
      <button
        type="button"
        className="motion-gallery-again"
        onClick={() => {
          setPlays(plays + 1);
        }}
      >
        Play again
      </button>
    </div>
  );
}

const LEVELS = ['T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];
const EDITED_AT = 2;
/** The two pieces a split leaves, each new. */
const NEW_LEVELS = ['T12', 'L1'];

const page: GalleryPage = {
  area: 'choreography',
  part: 'The four shared keyframes',
  states: [
    {
      name: 'breathe: one breath, three lights',
      show: () => (
        <div className="motion-gallery-row">
          <span className="motion-gallery-lit">Choose films</span>
          <div className="motion-gallery-choices">
            <span className="motion-gallery-choice">The upper one</span>
            <span className="motion-gallery-choice">The lower one</span>
          </div>
          <svg className="motion-gallery-film" width="96" height="72" viewBox="0 0 96 72">
            <rect className="motion-gallery-bone" x="18" y="8" width="60" height="56" rx="5" />
            <circle className="motion-gallery-ring" cx="48" cy="36" r="13" />
          </svg>
        </div>
      ),
    },
    {
      name: 'settle: guide lines, then cards',
      show: () => (
        <Replayed>
          <div className="motion-gallery-guide">
            <p className="motion-gallery-line motion-gallery-eyebrow" style={placeStyle(0)}>
              Step 1 of 4 · Films
            </p>
            <h2 className="motion-gallery-line motion-gallery-voice" style={placeStyle(1)}>
              Hang your four films.
            </h2>
            <p className="motion-gallery-line motion-gallery-body" style={placeStyle(2)}>
              Front, side, and one bending each way.
            </p>
            <div className="motion-gallery-card" style={placeStyle(0)}>
              Films: 0 of 4
            </div>
            <div className="motion-gallery-card" style={placeStyle(1)}>
              What happens to a film
            </div>
          </div>
        </Replayed>
      ),
    },
    {
      name: 'roll: names, each later the further from the edit',
      show: () => (
        <Replayed>
          <div className="motion-gallery-names">
            {placesFromEdit(LEVELS.length, EDITED_AT).map((place, level) => (
              <div key={LEVELS[level]} className="motion-gallery-name" data-edited={level === EDITED_AT ? 'yes' : 'no'}>
                <i />
                <span style={placeStyle(place)}>{LEVELS[level]}</span>
              </div>
            ))}
          </div>
        </Replayed>
      ),
    },
    {
      name: 'flash: a new outline, a new rung',
      show: () => (
        <Replayed>
          <div className="motion-gallery-row" data-conf="byyou">
            <svg className="motion-gallery-film" width="120" height="72" viewBox="0 0 120 72">
              <rect className="motion-gallery-bone" x="22" y="6" width="76" height="60" rx="5" />
              <path className="motion-gallery-outline-casing" d="M30 16L88 13L90 31L32 34Z" />
              <path className="motion-gallery-outline" d="M30 16L88 13L90 31L32 34Z" />
              <path className="motion-gallery-outline-casing" d="M32 40L90 37L92 55L34 58Z" />
              <path className="motion-gallery-outline" d="M32 40L90 37L92 55L34 58Z" />
            </svg>
            <div className="motion-gallery-rungs">
              {NEW_LEVELS.map((level) => (
                <div key={level} className="motion-gallery-rung">
                  <i className="motion-gallery-rung-frame" />
                  <span className="motion-gallery-rung-name">
                    <ConfidenceGlyph conf="byyou" />
                    {level}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </Replayed>
      ),
    },
  ],
};

export default page;
