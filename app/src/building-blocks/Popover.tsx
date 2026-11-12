import { useEffectEvent, useLayoutEffect, useRef, type ReactNode, type RefObject } from 'react';
import { useKey } from './keyboard/use-key';

interface PopoverProps {
  open: boolean;
  /** Asked to close: Escape, a press outside it, or focus moving on. */
  onClose: () => void;
  /** The control that opens it. A press there is not a press outside, and focus goes back to it. */
  anchor: RefObject<HTMLElement | null>;
  /** Its spoken name. */
  label: string;
  /** Its look and its place: the caller passes the design system's classes (the trust note's are `note glass`). */
  className?: string;
  children: ReactNode;
}

/** Focus goes back to the opener, unless the person has already put it somewhere else. */
function giveFocusBack(panel: HTMLElement, opener: HTMLElement | null) {
  const focused = document.activeElement;
  if (focused === null || focused === document.body || panel.contains(focused)) opener?.focus({ preventScroll: true });
}

/**
 * A panel beside the control that asked for it, which gets out of the way
 * easily. It is always on the page; `open` adds the class `open` and nothing
 * else, so the design system's own rules show it and hide it, each with its
 * fade. While closed it is hidden from assistive technology and takes no
 * focus or press. It has no look of its own.
 */
export function Popover({ open, onClose, anchor, label, className, children }: PopoverProps) {
  const panel = useRef<HTMLDivElement>(null);
  const close = useEffectEvent(onClose);

  const wasOpen = useRef(false);

  useKey('popover', ['Escape'], onClose, () => open);

  // Layout effects, so that as it closes, before the browser has moved focus
  // out of a panel that is now inert, they can still say whether focus was
  // inside it. Focus is given back in the effect itself and not in a cleanup:
  // React puts focus back where it was once it has changed the page, and the
  // panel is still on the page.
  useLayoutEffect(() => {
    const opened = panel.current;
    if (opened === null) return;
    if (!open) {
      if (wasOpen.current) giveFocusBack(opened, anchor.current);
      wasOpen.current = false;
      return;
    }
    wasOpen.current = true;
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
    };
  }, [open, anchor]);

  // Taken off the page while open: there is no later effect to give focus back, so it is given now.
  useLayoutEffect(() => {
    const element = panel.current;
    const opener = anchor.current;
    return () => {
      if (wasOpen.current && element !== null) giveFocusBack(element, opener);
    };
  }, [anchor]);

  const classes = [className, open ? 'open' : undefined].filter((each) => each !== undefined).join(' ');
  return (
    <div ref={panel} role="dialog" aria-label={label} tabIndex={-1} className={classes === '' ? undefined : classes} aria-hidden={open ? undefined : true} inert={!open}>
      {children}
    </div>
  );
}
