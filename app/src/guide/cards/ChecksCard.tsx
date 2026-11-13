import { useState } from 'react';
import type { RowState } from '@/look-and-feel/data-attributes';
import { CARD_WORDS } from '@/words/guide';
import { CardRows, type CardRow } from './CardRows';
import './ChecksCard.css';
import { GuideCard } from './GuideCard';

/** A check passes when the engine found nothing wrong, or a person settled it. */
const passes = (state: RowState): boolean => state === 'ok' || state === 'you';

interface ChecksCardProps {
  /** The standing checks, then any extra rows under them. `standingCheckRows` words the review's rows. */
  rows: readonly CardRow[];
}

/**
 * The checks that stand in view from the result on: how many vertebrae
 * were found, whether they are in order, whether the levels are named,
 * and any extra row a finding needs. A row that turns from open to
 * passing flashes once, so the eye is drawn to what a click just settled.
 */
export function ChecksCard({ rows }: ChecksCardProps) {
  const words = CARD_WORDS.checks;
  // The states last drawn, to tell a row that has just turned from one that was passing all along.
  const [drawn, setDrawn] = useState(rows);
  const [flashing, setFlashing] = useState<ReadonlySet<string>>(new Set());
  if (rows !== drawn) {
    const before = new Map(drawn.map((row) => [row.id, row.state]));
    const turned = rows.filter((row) => {
      const was = before.get(row.id);
      return was !== undefined && !passes(was) && passes(row.state);
    });
    setDrawn(rows);
    if (turned.length > 0) setFlashing(new Set([...flashing, ...turned.map((row) => row.id)]));
  }

  return (
    <GuideCard heading={words.heading} count={words.count(rows.filter((row) => passes(row.state)).length, rows.length)} className="checks-card">
      <CardRows
        rows={rows}
        flashing={flashing}
        onFlashed={(id) => {
          setFlashing((still) => new Set([...still].filter((other) => other !== id)));
        }}
      />
    </GuideCard>
  );
}
