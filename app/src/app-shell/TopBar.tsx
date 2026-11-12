import { useRef } from 'react';
import { Popover } from '@/building-blocks/Popover';
import { BrandMark } from '@/look-and-feel/BrandMark';
import { PromiseBadge } from '@/on-this-device/PromiseBadge';
import { PromisePopover } from '@/on-this-device/PromisePopover';
import type { RequestsSent } from '@/on-this-device/requests-sent';
import { ON_THIS_DEVICE_WORDS } from '@/words/on-this-device';
import { SHELL_WORDS } from '@/words/shell';
import type { RailStep } from './rail-steps';
import { StepSwitch } from './StepSwitch';

export interface TopBarProps {
  rail: readonly RailStep[];
  onGoToStep: (slug: string) => void;
  requests: RequestsSent;
  promiseOpen: boolean;
  onPromiseOpenChange: (open: boolean) => void;
}

/** The name, the journey and the promise. The note floats beneath the badge. */
export function TopBar({ rail, onGoToStep, requests, promiseOpen, onPromiseOpenChange }: TopBarProps) {
  const badge = useRef<HTMLButtonElement>(null);
  return (
    <>
      <header className="bar glass">
        <div className="brand">
          <BrandMark />
          <span className="brand-name">{SHELL_WORDS.appName}</span>
        </div>
        <StepSwitch rail={rail} onGo={onGoToStep} />
        <PromiseBadge ref={badge} open={promiseOpen} onOpenChange={onPromiseOpenChange} />
      </header>
      <Popover
        className="note glass"
        open={promiseOpen}
        anchor={badge}
        label={ON_THIS_DEVICE_WORDS.badge}
        onClose={() => { onPromiseOpenChange(false); }}
      >
        <PromisePopover requests={requests} />
      </Popover>
    </>
  );
}
