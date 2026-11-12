import { use } from 'react';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { Icon } from '@/look-and-feel/icons/Icon';
import type { IconName } from '@/look-and-feel/icons/icon-name';
import { LitStandsDown } from './lit-stands-down';

interface LitButtonProps {
  label: string;
  icon?: IconName | undefined;
  disabled?: boolean | undefined;
  asleep?: boolean;
  onPress: () => void;
}

/**
 * The next action: the one solid ink button on the page. It stands still.
 * Enter presses it from anywhere, unless a focused control keeps Enter
 * for itself. Holding Enter down presses it once.
 *
 * While a sheet with a lit button of its own is open it stands down to an
 * outline, and Enter is the sheet's.
 */
export function LitButton({ label, icon, disabled = false, asleep = false, onPress }: LitButtonProps) {
  const stoodDown = use(LitStandsDown);
  const run = () => { if (!disabled && !asleep) onPress(); };
  useKey(
    'lit',
    ['Enter'],
    (event) => {
      if (!event.repeat) run();
    },
    () => !stoodDown && !disabled && !asleep,
  );

  return (
    <button type="button" className={stoodDown ? 'secondary' : 'primary'} disabled={disabled} onClick={run}>
      {icon !== undefined && <Icon name={icon} />}
      <span>{label}</span>
    </button>
  );
}
