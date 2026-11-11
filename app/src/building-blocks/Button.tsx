import type { ComponentProps, ReactNode } from 'react';
import './Button.css';

interface ButtonProps extends Omit<ComponentProps<'button'>, 'type' | 'disabled' | 'onClick'> {
  /** `standard` has a face and an edge; `text` is the words alone, for a quiet action. */
  look?: 'standard' | 'text';
  /** `large` stands a standard button beside a lit one. */
  size?: 'regular' | 'large';
  /** Cannot be pressed, and is left out of the tab order. */
  disabled?: boolean;
  /** Its press is being carried out: it keeps its size and its focus, shows it is working, and takes no second press. */
  busy?: boolean;
  onPress?: () => void;
  /** The words, with an icon before them if the action has one. */
  children: ReactNode;
}

/**
 * A button that is not the next action. The one lit button is the
 * guide's, and nothing here wears its light.
 */
export function Button({ look = 'standard', size = 'regular', disabled = false, busy = false, onPress, className, children, ...rest }: ButtonProps) {
  return (
    <button
      {...rest}
      type="button"
      className={className === undefined ? 'button' : `button ${className}`}
      data-look={look}
      data-size={size}
      disabled={disabled}
      aria-busy={busy || undefined}
      aria-disabled={busy || undefined}
      onClick={busy ? undefined : onPress}
    >
      <span className="button-content">{children}</span>
      {busy && <span className="button-working" aria-hidden="true" />}
    </button>
  );
}
