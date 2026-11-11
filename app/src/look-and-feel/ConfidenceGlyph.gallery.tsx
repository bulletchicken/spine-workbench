import type { GalleryPage } from '@/gallery/gallery-page';
import type { GlossaryTerm } from '@/words/glossary';
import { ConfidenceGlyph, ConfidenceGlyphShapes } from './ConfidenceGlyph';
import type { Conf } from './data-attributes';
import './look-and-feel.gallery.css';

const STATES: readonly { conf: Conf; word: GlossaryTerm | 'Not counted' }[] = [
  { conf: 'sure', word: 'Sure' },
  { conf: 'look', word: 'Worth a look' },
  { conf: 'needs', word: 'Needs you' },
  { conf: 'byyou', word: 'Set by you' },
  { conf: 'sacrum', word: 'Not counted' },
];

/** What a glyph on a film may lie on: bright bone, bone, and the dark of the film around it. */
const GROUNDS: readonly { ground: string; levels: readonly string[] }[] = [
  { ground: 'look-gallery-film-bright', levels: ['T3', 'T4', 'T5', 'T6', 'T7'] },
  { ground: 'look-gallery-film-bone', levels: ['T8', 'T9', 'T10', 'T11', 'T12'] },
  { ground: 'look-gallery-film-soft', levels: ['L1', 'L2', 'L3', 'L4', 'L5'] },
];

const page: GalleryPage = {
  area: 'look-and-feel',
  part: 'ConfidenceGlyph',
  states: [
    {
      name: 'The five glyphs on paper, each with its word',
      show: () => (
        <div className="look-gallery-set">
          {STATES.map(({ conf, word }) => (
            <span key={conf} className="look-gallery-worded">
              <ConfidenceGlyph conf={conf} />
              {word}
            </span>
          ))}
        </div>
      ),
    },
    {
      name: 'Large, to check the drawing',
      show: () => (
        <div className="look-gallery-set">
          {STATES.map(({ conf }) => (
            <span key={conf} className="look-gallery-item">
              <ConfidenceGlyph conf={conf} className="look-gallery-large" />
              <span className="look-gallery-name">{conf}</span>
            </span>
          ))}
        </div>
      ),
    },
    {
      name: 'On a film, cased, beside a level name',
      show: () => (
        <svg className="look-gallery-film" data-ground="film" width="330" height="114" viewBox="0 0 330 114">
          {GROUNDS.map(({ ground, levels }, row) => (
            <g key={ground} transform={`translate(0 ${String(row * 38)})`}>
              <rect className={ground} width="330" height="38" />
              {STATES.map(({ conf }, index) => (
                <g key={conf} data-conf={conf}>
                  <g transform={`translate(${String(22 + index * 66)} 19)`}>
                    <ConfidenceGlyphShapes conf={conf} cased />
                  </g>
                  <text className="look-gallery-film-label" x={32 + index * 66} y="23">
                    {levels[index]}
                  </text>
                </g>
              ))}
            </g>
          ))}
        </svg>
      ),
    },
  ],
};

export default page;
