import { useId, type ReactNode } from 'react';
import './GuideCard.css';

interface GuideCardProps {
  /** Set in small capitals. */
  heading: string;
  /** What stands at the heading's right, set as data: "2 of 3 pass". */
  count?: string | undefined;
  className?: string | undefined;
  children: ReactNode;
}

/**
 * The frame every card shares. On screen a card is a ruled section: a
 * hairline across the guide, a small heading with its count, and whatever
 * the card holds. It arrives once, in its turn after the lines above.
 */
export function GuideCard({ heading, count, className, children }: GuideCardProps) {
  const headingId = useId();
  return (
    <section className={className === undefined ? 'guide-card' : `guide-card ${className}`} aria-labelledby={headingId}>
      <h2 className="guide-card-head" id={headingId}>
        <span>{heading}</span>
        {count !== undefined && <b className="guide-card-count">{count}</b>}
      </h2>
      {children}
    </section>
  );
}
