import { useRef } from 'react';
import { Popover } from '@/building-blocks/Popover';
import { Icon } from '@/look-and-feel/icons/Icon';
import { ON_THIS_DEVICE_WORDS as WORDS } from '@/words/on-this-device';
import './PromiseBadge.css';
import { PromisePopover } from './PromisePopover';
import type { RequestsSent } from './requests-sent';

interface PromiseBadgeProps {
  requests: RequestsSent;
  /** The popover is open. */
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/**
 * "On this device only · 0 sent", always in the top bar. A press opens
 * the popover that makes the promise checkable; Escape, a press outside
 * or focus moving on closes it. The popover is placed against the badge's
 * nearest positioned ancestor, which in the app is the top bar.
 */
export function PromiseBadge({ requests, open, onOpenChange }: PromiseBadgeProps) {
  const badge = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button
        ref={badge}
        type="button"
        className="promise-badge"
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => {
          onOpenChange(!open);
        }}
      >
        <Icon name="on-device" />
        <span>{WORDS.badge}</span>
        <b className="promise-badge-sent">{WORDS.sent(requests.sent)}</b>
      </button>
      <Popover
        open={open}
        onClose={() => {
          onOpenChange(false);
        }}
        anchor={badge}
        label={WORDS.badge}
        className="promise-popover"
      >
        <PromisePopover requests={requests} />
      </Popover>
    </>
  );
}
