import { use } from 'react';
import { Kbd } from '@/building-blocks/Kbd';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { Icon } from '@/look-and-feel/icons/Icon';
import type { IconName } from '@/look-and-feel/icons/icon-name';
import './LitButton.css';
import { LitStandsDown } from './lit-stands-down';

interface LitButtonProps {
  label: string;
  icon?: IconName | undefined;
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
export function LitButton({ label, icon, onPress }: LitButtonProps) {
  const stoodDown = use(LitStandsDown);
  useKey(
    'lit',
    ['Enter'],
    (event) => {
      if (!event.repeat) onPress();
    },
    () => !stoodDown,
  );

  return (
    <button type="button" className="lit-button" data-stood-down={stoodDown || undefined} onClick={onPress}>
      {icon !== undefined && <Icon name={icon} />}
      <span>{label}</span>
      <Kbd keys={['Enter']} />
    </button>
  );
}
