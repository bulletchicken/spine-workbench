import type { CSSProperties } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { Icon } from './icons/Icon';
import './look-and-feel.gallery.css';

// Each role is shown in the token's own `font`, so the page shows what the tokens say and
// nothing of its own. The gallery is for development only and is never built into the app.
const ROLE: Record<string, CSSProperties> = {
  display: { font: 'var(--type-display)', letterSpacing: 'var(--track-display)' },
  heading: { font: 'var(--type-heading)', letterSpacing: 'var(--track-heading)' },
  body: { font: 'var(--type-body)' },
  bodyMedium: { font: 'var(--type-body-medium)' },
  small: { font: 'var(--type-small)', color: 'var(--ink-2)' },
  smallMedium: { font: 'var(--type-small-medium)', color: 'var(--ink-2)' },
  caption: { font: 'var(--type-caption)', letterSpacing: 'var(--track-caption)', color: 'var(--ink-2)' },
  mark: { font: 'var(--weight-medium) var(--size-mark) / 1 var(--font-mono)' },
};

const page: GalleryPage = {
  area: 'look-and-feel',
  part: 'Type specimen',
  states: [
    {
      name: 'Geist: every role, from the start sentence to a film’s name',
      show: () => (
        <div className="look-gallery-type">
          <div className="look-gallery-type-row">
            <span>display 44/48</span>
            <span style={ROLE.display}>Add the four films.</span>
          </div>
          <div className="look-gallery-type-row">
            <span>heading 18/24</span>
            <span style={ROLE.heading}>17 vertebrae found. 2 need a look.</span>
          </div>
          <div className="look-gallery-type-row">
            <span>body 15/22</span>
            <span style={ROLE.body}>Your films never leave this computer.</span>
          </div>
          <div className="look-gallery-type-row">
            <span>body medium</span>
            <span style={ROLE.bodyMedium}>Find the spine</span>
          </div>
          <div className="look-gallery-type-row">
            <span>small 13/18</span>
            <span style={ROLE.small}>You can find the spine now, or add the others.</span>
          </div>
          <div className="look-gallery-type-row">
            <span>small medium</span>
            <span style={ROLE.smallMedium}>Start over</span>
          </div>
          <div className="look-gallery-type-row">
            <span>caption 12/16</span>
            <span style={ROLE.caption}>Bending left</span>
          </div>
          <div className="look-gallery-type-row">
            <span>weights</span>
            <span className="look-gallery-weights" style={ROLE.body}>
              <span>Regular 400</span>
              <span>Medium 500</span>
              <span>Semibold 600</span>
            </span>
          </div>
        </div>
      ),
    },
    {
      name: 'Geist Mono: numbers and vertebra names only, one width a figure',
      show: () => (
        <div className="look-gallery-type">
          <div className="look-gallery-type-row">
            <span>in a heading</span>
            <span style={ROLE.heading}>
              <span className="num">17</span> vertebrae found.
            </span>
          </div>
          <div className="look-gallery-type-row">
            <span>in small</span>
            <span style={ROLE.small}>
              <span className="num">9 of 17</span>
            </span>
          </div>
          <div className="look-gallery-type-row">
            <span>on a film, 11</span>
            <span style={ROLE.mark}>T1 T12 L1 L5</span>
          </div>
          <div className="look-gallery-type-row">
            <span>figures</span>
            <span className="look-gallery-figures-column num">
              <span>1000.00 mm</span>
              <span>111.11 mm</span>
              <span>80.08 mm</span>
            </span>
          </div>
          <div className="look-gallery-type-row">
            <span>told apart</span>
            <span style={ROLE.heading}>
              <span className="num">0O 1lI 5S 8B</span>
            </span>
          </div>
          <div className="look-gallery-type-row">
            <span>key hints</span>
            <span className="look-gallery-keys">
              <kbd>
                <Icon name="arrow-left" />
              </kbd>
              <kbd>
                <Icon name="arrow-right" />
              </kbd>
              <kbd>
                <Icon name="arrow-up" />
              </kbd>
              <kbd>
                <Icon name="arrow-down" />
              </kbd>
              <kbd>
                <Icon name="command" />Z
              </kbd>
              <kbd>Ctrl+Z</kbd>
              <kbd>Enter</kbd>
              <kbd>Esc</kbd>
            </span>
          </div>
        </div>
      ),
    },
    {
      name: 'One ink scale on the page',
      show: () => (
        <div className="look-gallery-inks">
          <span style={{ ...ROLE.body, color: 'var(--ink)' }}>Ink: sentences, the brand</span>
          <span style={{ ...ROLE.body, color: 'var(--ink-2)' }}>Ink 2: helpers, names, quiet buttons</span>
          <span style={{ ...ROLE.body, color: 'var(--ink-3)' }}>Ink 3: a disabled button’s words only</span>
          <span style={{ ...ROLE.body, color: 'var(--ink-faint)' }}>Faint: a step not yet reached</span>
        </div>
      ),
    },
  ],
};

export default page;
