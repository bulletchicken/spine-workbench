import type { GalleryPage } from '@/gallery/gallery-page';
import type { GlossaryTerm } from '@/words/glossary';
import { ConfidenceGlyph, ConfidenceGlyphShapes } from './ConfidenceGlyph';
import type { Conf } from './data-attributes';
import './look-and-feel.gallery.css';

const STATES: readonly { conf: Conf; word: GlossaryTerm | 'Not counted'; level: string; count: number }[] = [
  { conf: 'sure', word: 'Sure', level: 'T8', count: 13 },
  { conf: 'look', word: 'Worth a look', level: 'T1', count: 2 },
  { conf: 'needs', word: 'Needs you', level: 'T3', count: 2 },
  { conf: 'byyou', word: 'Set by you', level: 'T12', count: 0 },
  { conf: 'sacrum', word: 'Not counted', level: 'L6', count: 0 },
];

/** One vertebra's outline, as a slightly leaning box, for the row it is drawn in. */
function outlineAt(row: number): string {
  const top = 14 + row * 44;
  return `M96 ${String(top + 3)}L150 ${String(top)}L152 ${String(top + 30)}L98 ${String(top + 33)}Z`;
}

const page: GalleryPage = {
  area: 'look-and-feel',
  part: 'Confidence: colour and line',
  states: [
    {
      name: 'An outline on a film, in each state',
      show: () => (
        <svg className="look-gallery-film" width="250" height="236" viewBox="0 0 250 236">
          <rect className="look-gallery-film-soft" width="250" height="236" />
          <rect className="look-gallery-film-bone" x="92" y="8" width="64" height="220" rx="6" />
          <rect className="look-gallery-film-bright" x="92" y="96" width="64" height="44" />
          {STATES.map(({ conf, level }, row) => (
            <g key={conf} data-conf={conf}>
              <path className="look-gallery-outline-casing" d={outlineAt(row)} />
              <path className="look-gallery-outline" d={outlineAt(row)} />
              <g transform={`translate(${String(84 - level.length * 7.1 - 8)} ${String(30 + row * 44)})`}>
                <ConfidenceGlyphShapes conf={conf} cased />
              </g>
              <text className="look-gallery-film-label" x="84" y={34 + row * 44} textAnchor="end">
                {level}
              </text>
            </g>
          ))}
        </svg>
      ),
    },
    {
      name: 'A rung of the ladder, in each state',
      show: () => (
        <div className="look-gallery-rungs">
          {STATES.map(({ conf, level }) => (
            <div key={conf} className="look-gallery-rung" data-conf={conf}>
              <i className="look-gallery-rung-bar" />
              <i className="look-gallery-rung-frame" />
              <span className="look-gallery-rung-name">
                <ConfidenceGlyph conf={conf} />
                {level}
              </span>
            </div>
          ))}
        </div>
      ),
    },
    {
      name: 'A legend chip, in each counted state',
      show: () => (
        <div className="look-gallery-legend">
          {STATES.filter(({ conf }) => conf !== 'sacrum').map(({ conf, word, count }) => (
            <div key={conf} className="look-gallery-chip" data-conf={conf}>
              <span className="look-gallery-worded">
                <ConfidenceGlyph conf={conf} />
                {word}
              </span>
              <span className="look-gallery-chip-count">{count}</span>
              <span className="look-gallery-chip-line" />
            </div>
          ))}
        </div>
      ),
    },
  ],
};

export default page;
