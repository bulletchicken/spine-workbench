import { useLayoutEffect, useRef, type ReactNode } from 'react';
import { Kbd } from '../Kbd';
import type { Said, ToastAction } from './say';
import './Toast.css';

interface ToastProps {
  /** The line to show. While `shown` is false it is the line that is fading out. */
  said: Said | null;
  shown: boolean;
  /** The icon the line opens with, for its tone. */
  mark: ReactNode;
  /** The action was pressed. */
  onAction: () => void;
  /** It has stayed its time, with no pointer on it and no focus in it. */
  onStayed: () => void;
}

/**
 * The toast's one action, and the only thing in a toast that takes focus.
 * It goes away with the toast: when it is pressed, or when a newer line
 * with no action takes the toast's place. If it has focus then, focus
 * goes back to where it was before it came to the toast, and not to the
 * top of the page.
 */
function ActionButton({ action, shown, onPress }: { action: ToastAction; shown: boolean; onPress: () => void }) {
  const button = useRef<HTMLButtonElement>(null);
  const cameFrom = useRef<HTMLElement | null>(null);
  const giveFocusBack = (from: HTMLButtonElement | null) => {
    if (from === null || document.activeElement !== from) return;
    const before = cameFrom.current;
    cameFrom.current = null;
    if (before?.isConnected === true) before.focus({ preventScroll: true });
  };

  // Put away while it is still drawn: the pill fades with the button in it.
  useLayoutEffect(() => {
    if (!shown) giveFocusBack(button.current);
  }, [shown]);
  // Taken off the page. The element is read now: the ref is empty by then.
  useLayoutEffect(() => {
    const drawn = button.current;
    return () => {
      giveFocusBack(drawn);
    };
  }, []);

  return (
    <button
      ref={button}
      type="button"
      className="toast-action"
      onClick={onPress}
      onFocus={(event) => {
        if (event.relatedTarget instanceof HTMLElement) cameFrom.current = event.relatedTarget;
      }}
    >
      {action.label}
      {action.keys !== undefined && <Kbd keys={action.keys} look="bare" />}
    </button>
  );
}

/**
 * One line on a glass pill, with at most one action. How long it stays is
 * `--t-toast-stay`, kept by a CSS animation on a hidden clock, so the
 * time lives in one place and the clock stops while the pointer is on the
 * toast or focus is inside it.
 *
 * The line is drawn here and spoken by Toasts, so the pill can fade away
 * without the status message going with it.
 */
export function Toast({ said, shown, mark, onAction, onStayed }: ToastProps) {
  return (
    <div className="toast" data-shown={shown || undefined} data-action={said?.action != null || undefined}>
      {said !== null && (
        <>
          <span className="toast-mark" data-tone={said.tone} aria-hidden="true">
            {mark}
          </span>
          <span className="toast-line" aria-hidden="true">
            {said.line}
          </span>
          {said.action !== null && <ActionButton action={said.action} shown={shown} onPress={onAction} />}
          {shown && <span key={said.id} className="toast-clock" onAnimationEnd={onStayed} />}
        </>
      )}
    </div>
  );
}
