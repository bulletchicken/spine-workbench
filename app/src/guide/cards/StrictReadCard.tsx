import type { RowState } from '@/look-and-feel/data-attributes';
import { CARD_WORDS } from '@/words/guide';
import { CardRows } from './CardRows';
import { GuideCard } from './GuideCard';

/** One thing checked while a case file is read. */
export interface StrictReadLine {
  /** "It is a spine-case file, version 1". */
  line: string;
  /** `ok` once it has been checked, `busy` while it is, `wait` before, `bad` where the file was refused. */
  state: RowState;
}

interface StrictReadCardProps {
  lines: readonly StrictReadLine[];
}

/** The lines of a case file being read strictly, ticking in turn. It ticks only what it is told was checked. */
export function StrictReadCard({ lines }: StrictReadCardProps) {
  const words = CARD_WORDS.strictRead;
  const read = lines.filter((line) => line.state === 'ok').length;
  return (
    <GuideCard heading={words.heading} count={words.count(read, lines.length)}>
      <CardRows rows={lines.map((line) => ({ id: line.line, state: line.state, title: line.line }))} />
    </GuideCard>
  );
}
