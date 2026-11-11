import type { ComponentProps, ReactNode } from 'react';
import './IconButton.css';
import { Tooltip } from './Tooltip';

type TooltipPlace = Pick<ComponentProps<typeof Tooltip>, 'side' | 'align'>;

interface IconButtonProps extends Omit<ComponentProps<'button'>, 'type' | 'disabled' | 'onClick' | 'aria-label'> {
  /** Its name, as it is spoken: "Undo". */
  label: string;
  /** What its tooltip says when that is more than its name: "Undo: Split T12", "Nothing to undo". */
  tip?: ReactNode;
  /** Where the tooltip opens. */
  tipSide?: TooltipPlace['side'];
  tipAlign?: TooltipPlace['align'];
  /** `round` is for a row of small controls in a strip, such as playback. */
  shape?: 'square' | 'round';
  /** Cannot be pressed. It stays in the tab order, so its tooltip can still say why. */
  disabled?: boolean;
  onPress?: () => void;
  /** The icon. */
  children: ReactNode;
}

/**
 * A button that shows an icon and no words. It is never an icon alone: it
 * has a spoken name and a tooltip, and the tooltip can name the exact
 * action a press would do.
 */
export function IconButton({
  label,
  tip,
  tipSide = 'below',
  tipAlign = 'centre',
  shape = 'square',
  disabled = false,
  onPress,
  className,
  children,
  ...rest
}: IconButtonProps) {
  return (
    <Tooltip tip={tip ?? label} side={tipSide} align={tipAlign} spoken={tip !== undefined}>
      <button
        {...rest}
        type="button"
        className={className === undefined ? 'icon-button' : `icon-button ${className}`}
        data-shape={shape}
        aria-label={label}
        aria-disabled={disabled || undefined}
        onClick={disabled ? undefined : onPress}
      >
        {children}
      </button>
    </Tooltip>
  );
}
