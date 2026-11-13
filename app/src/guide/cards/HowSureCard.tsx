import { ConfidenceGlyph } from '@/look-and-feel/ConfidenceGlyph';
import { CARD_WORDS } from '@/words/guide';
import { CONF_WORDS, UNCALIBRATED_NOTE } from '@/words/how-sure';
import { GuideCard } from './GuideCard';
import './HowSureCard.css';

/** The four states a counted level can be in, in the order the card shows them. */
const STATES = ['sure', 'look', 'needs', 'byyou'] as const;

interface HowSureCardProps {
  /** How many counted levels are in each state. */
  counts: Readonly<Record<(typeof STATES)[number], number>>;
  /** The engine's confidence has been checked against real films. Until it has, the card says so. */
  calibrated: boolean;
  /** Close the card by saying that colour, line and mark say the same thing: for once the marks are on the films to compare. */
  withKey?: boolean | undefined;
}

/**
 * The key and the tally in one: four ruled cells, each with a state's
 * glyph, its word, its count and a sample of its line. A cell is a legend
 * chip: it carries `data-conf`, and confidence.css gives it its colour and
 * line as it gives them to a rung.
 */
export function HowSureCard({ counts, calibrated, withKey = false }: HowSureCardProps) {
  const words = CARD_WORDS.howSure;
  const notes = [...(withKey ? [words.sameThing] : []), ...(calibrated ? [] : [UNCALIBRATED_NOTE])];
  return (
    <GuideCard heading={words.heading} className="how-sure-card">
      <dl className="how-sure">
        {STATES.map((state) => (
          <div key={state} className="how-sure-chip" data-conf={state}>
            <dt className="how-sure-word">
              <ConfidenceGlyph conf={state} />
              {CONF_WORDS[state]}
            </dt>
            <dd className="how-sure-count">{counts[state]}</dd>
          </div>
        ))}
      </dl>
      {notes.length > 0 && <p className="guide-card-note how-sure-note">{notes.join(' ')}</p>}
    </GuideCard>
  );
}
