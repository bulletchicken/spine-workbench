import type { Ref } from 'react';
import { ON_THIS_DEVICE_WORDS as WORDS } from '@/words/on-this-device';

interface PromiseBadgeProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  ref?: Ref<HTMLButtonElement>;
}

/** The promise stays in view; its count lives in the note it opens. */
export function PromiseBadge({ open, onOpenChange, ref }: PromiseBadgeProps) {
  return (
    <button ref={ref} type="button" className="badge" aria-haspopup="dialog" aria-expanded={open} onClick={() => { onOpenChange(!open); }}>
      <svg viewBox="0 0 18 18" aria-hidden="true">
        <rect x="3.75" y="8" width="10.5" height="7" rx="2" />
        <path d="M6 8V5.75a3 3 0 0 1 6 0V8" />
      </svg>
      {WORDS.badge}
    </button>
  );
}
