import type { GalleryPage } from '@/gallery/gallery-page';
import { Icon } from './icons/Icon';
import './look-and-feel.gallery.css';

const page: GalleryPage = {
  area: 'look-and-feel',
  part: 'Type specimen',
  states: [
    {
      name: 'The guide’s type, at first launch',
      show: () => (
        <div className="look-gallery-guide">
          <p className="look-gallery-caps">Step 1 of 4 · Films</p>
          <h2 className="look-gallery-voice">Hang your four films.</h2>
          <p className="look-gallery-body">
            Front, side, and one bending each way. Drop the files anywhere on this window, or choose them.
          </p>
        </div>
      ),
    },
    {
      name: 'The guide’s type, at the result',
      show: () => (
        <div className="look-gallery-guide">
          <p className="look-gallery-caps">Step 3 of 4 · Find</p>
          <h2 className="look-gallery-voice">Found 17. Four need a look.</h2>
          <p className="look-gallery-body">
            Blue outlines are sure. Yellow and coral mark where the films were hard to read. We’ll go through those one at a time.
          </p>
        </div>
      ),
    },
    {
      name: 'Voice: Instrument Serif',
      show: () => (
        <div className="look-gallery-type">
          <div className="look-gallery-type-row">
            <span>voice 32/36</span>
            <span className="look-gallery-voice">Hang your four films.</span>
          </div>
          <div className="look-gallery-type-row">
            <span>voice 32/36</span>
            <span className="look-gallery-voice">Found 17. Four need a look.</span>
          </div>
          <div className="look-gallery-type-row">
            <span>title 28/32</span>
            <span className="look-gallery-voice-title">Nothing leaves this computer.</span>
          </div>
          <div className="look-gallery-type-row">
            <span>italic</span>
            <span className="look-gallery-voice-title">
              <i>Which vertebra is L5?</i>
            </span>
          </div>
          <div className="look-gallery-type-row">
            <span>hero 48/50</span>
            <span className="look-gallery-hero">L R ?</span>
          </div>
        </div>
      ),
    },
    {
      name: 'Interface: Geist',
      show: () => (
        <div className="look-gallery-type">
          <div className="look-gallery-type-row">
            <span>lead 16/24</span>
            <span className="look-gallery-lead">Choose films</span>
          </div>
          <div className="look-gallery-type-row">
            <span>body 14/21</span>
            <span className="look-gallery-body">
              Front, side, and one bending each way. Drop the files anywhere on this window, or choose them.
            </span>
          </div>
          <div className="look-gallery-type-row">
            <span>small 12/18</span>
            <span className="look-gallery-small">Counting from the first rib and from the sacrum disagree</span>
          </div>
          <div className="look-gallery-type-row">
            <span>micro 11/16</span>
            <span className="look-gallery-caps">Step 1 of 4 · Films</span>
          </div>
          <div className="look-gallery-type-row">
            <span>weights</span>
            <span className="look-gallery-weights">
              <span>Regular 400</span>
              <span>Medium 500</span>
              <span>Semibold 600</span>
            </span>
          </div>
        </div>
      ),
    },
    {
      name: 'Data: Geist Mono, one width a figure, a slashed zero',
      show: () => (
        <div className="look-gallery-type">
          <div className="look-gallery-type-row">
            <span>title 20/26</span>
            <span className="look-gallery-title-number">13 · 2 · 2 · 0</span>
          </div>
          <div className="look-gallery-type-row">
            <span>small 12/18</span>
            <span className="look-gallery-data">C7 T1 T12 L5 · 0.45 mm/px · 312% · spine-case-2026-11-10.spinecase</span>
          </div>
          <div className="look-gallery-type-row">
            <span>figures</span>
            <span className="look-gallery-figures-column">
              <span>1000.00 mm</span>
              <span>111.11 mm</span>
              <span>80.08 mm</span>
            </span>
          </div>
          <div className="look-gallery-type-row">
            <span>told apart</span>
            <span className="look-gallery-title-number">0O 1lI 5S 8B</span>
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
              <kbd>
                <Icon name="shift" />
                <Icon name="command" />Z
              </kbd>
              <kbd>
                <Icon name="option" />
                <Icon name="arrow-up" />
              </kbd>
              <kbd>Ctrl+Z</kbd>
              <kbd>Enter</kbd>
              <kbd>Esc</kbd>
              <kbd>V</kbd>
            </span>
          </div>
        </div>
      ),
    },
    {
      name: 'The four inks, on the room and on a card',
      show: () => (
        <>
          <div className="look-gallery-inks">
            <span className="look-gallery-ink-1">Ink 1: text</span>
            <span className="look-gallery-ink-2">Ink 2: secondary text</span>
            <span className="look-gallery-ink-3">Ink 3: captions, labels</span>
            <span className="look-gallery-ink-4">Ink 4: 24 px and larger only</span>
          </div>
          <div className="look-gallery-inks look-gallery-card">
            <span className="look-gallery-ink-1">Ink 1: text</span>
            <span className="look-gallery-ink-2">Ink 2: secondary text</span>
            <span className="look-gallery-ink-3">Ink 3: captions, labels</span>
            <span className="look-gallery-ink-4">Ink 4: 24 px and larger only</span>
          </div>
        </>
      ),
    },
  ],
};

export default page;
