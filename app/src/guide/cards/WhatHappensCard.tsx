import { CARD_WORDS } from '@/words/guide';
import { CardRows } from './CardRows';
import { GuideCard } from './GuideCard';

/** What happens to a film: what is kept, what is dropped, and that it is sent nowhere. */
export function WhatHappensCard() {
  const words = CARD_WORDS.whatHappens;
  return (
    <GuideCard heading={words.heading}>
      <CardRows rows={words.rows.map((row) => ({ id: row.title, state: 'ok', title: row.title, lines: [row.line] }))} />
    </GuideCard>
  );
}
