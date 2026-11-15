import type { ComponentProps, ReactNode } from 'react';
import './Button.css';

interface ButtonProps extends Omit<ComponentProps<'button'>, 'type' | 'disabled' | 'onClick'> {
  /** `standard` is the design's secondary button, a white block with a hairline edge; `text` is its quiet button, the words alone. */
  look?: 'standard' | 'text';
  /** `large` is the design's size for the secondary button; `regular` is the smaller one earlier parts ask for. */
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
 * A button that is not the next action. It wears the design system's class
 * for its look (`quiet` or `secondary`, from components.css) beside `button`,
 * a hook other stylesheets place it by. The one black button, the next step
 * of the moment, is the guide's, and nothing here is ever black.
 */
export function Button({ look = 'standard', size = 'regular', disabled = false, busy = false, onPress, className, children, ...rest }: ButtonProps) {
  return (
    <button
      {...rest}
      type="button"
      className={['button', look === 'text' ? 'quiet' : 'secondary', className].filter((each) => each !== undefined).join(' ')}
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
