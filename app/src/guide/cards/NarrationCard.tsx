import { Icon } from '@/look-and-feel/icons/Icon';
import { CARD_WORDS } from '@/words/guide';
import { GuideCard } from './GuideCard';
import './NarrationCard.css';

/** One beat of the pass in plain words: what it is doing, and a sentence more for while it does it. */
export interface NarrationLine {
  /** "Looking for bone". */
  title: string;
  /** "Edges, ridges and pedicle rings are picked out on both films." */
  line: string;
}

interface NarrationCardProps {
  lines: readonly NarrationLine[];
  /** The line being played now, counted from 0: it is opened. `null` before the first and after the last. */
  live: number | null;
  /** How many lines, from the top, are finished and ticked. */
  finished: number;
}

/**
 * What the pass is doing, as one plain line per beat. The live line is
 * opened to its explanation; finished lines tick. It is given where the
 * pass is and never looks at a clock itself.
 */
export function NarrationCard({ lines, live, finished }: NarrationCardProps) {
  const words = CARD_WORDS.narration;
  return (
    <GuideCard heading={words.heading} className="narration-card">
      <ol className="narration">
        {lines.map((line, at) => (
          <li
            key={line.title}
            className="narration-line"
            data-on={at === live || undefined}
            data-done={at < finished || undefined}
            aria-current={at === live ? 'step' : undefined}
          >
            <span className="narration-number" aria-hidden="true">
              {at < finished ? <Icon name="check" /> : at + 1}
            </span>
            <div>
              <b className="narration-title">{line.title}</b>
              <span className="narration-more" aria-hidden={at !== live}>
                <span>{line.line}</span>
              </span>
            </div>
          </li>
        ))}
      </ol>
      <p className="guide-card-note">{words.note}</p>
    </GuideCard>
  );
}
