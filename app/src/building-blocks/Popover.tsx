import { useEffectEvent, useLayoutEffect, useRef, type ReactNode, type RefObject } from 'react';
import { useKey } from './keyboard/use-key';
import './Popover.css';

interface PopoverProps {
  open: boolean;
  /** Asked to close: Escape, a press outside it, or focus moving on. */
  onClose: () => void;
  /** The control that opens it. A press there is not a press outside, and focus goes back to it. */
  anchor: RefObject<HTMLElement | null>;
  /** Its spoken name. */
  label: string;
  /** Which side of its place it opens on, and which edge it lines up with. */
  side?: 'below' | 'above';
  align?: 'start' | 'centre' | 'end';
  className?: string;
  children: ReactNode;
}

/**
 * A glass panel that opens beside the control that asked for it and gets
 * out of the way easily. It is placed against its nearest positioned
 * ancestor, so wrap the control and the popover in one positioned
 * element, or give it a place of its own through `className`.
 */
export function Popover({ open, ...rest }: PopoverProps) {
  return open ? <OpenPopover {...rest} /> : null;
}

function OpenPopover({ onClose, anchor, label, side = 'below', align = 'end', className, children }: Omit<PopoverProps, 'open'>) {
  const panel = useRef<HTMLDivElement>(null);
  const close = useEffectEvent(onClose);

  useKey('popover', ['Escape'], onClose);

  // A layout effect, so that on the way out the panel is still on the
  // page and can say whether focus was inside it.
  useLayoutEffect(() => {
    const opened = panel.current;
    if (opened === null) return;
    const opener = anchor.current;
    opened.focus({ preventScroll: true });

    const isOutside = (at: EventTarget | null) => at instanceof Node && !opened.contains(at) && opener?.contains(at) !== true;
    const onPointerDown = (event: PointerEvent) => {
      if (isOutside(event.target)) close();
    };
    const onFocusOut = (event: FocusEvent) => {
      if (isOutside(event.relatedTarget)) close();
    };
    document.addEventListener('pointerdown', onPointerDown, true);
    opened.addEventListener('focusout', onFocusOut);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown, true);
      opened.removeEventListener('focusout', onFocusOut);
      // Focus goes back to the opener unless the person has already put it somewhere else.
      const focused = document.activeElement;
      if (focused === null || focused === document.body || opened.contains(focused)) opener?.focus({ preventScroll: true });
    };
  }, [anchor]);

  return (
    <div
      ref={panel}
      role="dialog"
      aria-label={label}
      tabIndex={-1}
      className={className === undefined ? 'popover' : `popover ${className}`}
      data-side={side}
      data-align={align}
    >
      {children}
    </div>
  );
}
