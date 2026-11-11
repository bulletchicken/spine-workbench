import { useEffect, useLayoutEffect, useRef, useState, type AnimationEvent, type KeyboardEvent, type ReactNode } from 'react';
import './Dialog.css';
import { stopKeysAt } from './keyboard/keyboard';
import { useKey } from './keyboard/use-key';

interface DialogProps {
  open: boolean;
  /** Asked to close: Escape. */
  onClose: () => void;
  /** Its spoken name: the id of its heading, or the name itself when it has no heading. */
  labelledBy?: string;
  label?: string;
  /** For the panel: its width, and how its inside is laid out. */
  className?: string;
  children: ReactNode;
}

const CAN_TAKE_FOCUS =
  'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

/** What Tab stops at inside `panel`, in order. */
function stopsIn(panel: HTMLElement): HTMLElement[] {
  return [...panel.querySelectorAll<HTMLElement>(CAN_TAKE_FOCUS)];
}

/**
 * A sheet of paper over a veil, for the one thing that must be answered
 * before going on.
 *
 * The veil covers the dialog's nearest positioned ancestor and nothing
 * else: mount the dialog inside the region it should cover (the bench,
 * for the case sheet) and the rest of the window stays as it was.
 *
 * While it is open, focus stays inside it: it starts on the first control
 * (or the one marked `data-autofocus`), Tab goes round, and focus that
 * lands anywhere else is brought back, unless it went into another dialog
 * or popover opened over this one. A press on the veil does nothing, and
 * no key reaches a part behind it: only `always` and `sheet` are asked,
 * so undo and save still answer and a tool's letter does not. Escape asks
 * to close, and focus returns to where it was. On the way out the sheet
 * stays mounted until it has faded.
 */
export function Dialog({ open, ...rest }: DialogProps) {
  // Mounted from the moment it opens until its leaving animation ends.
  const [present, setPresent] = useState(open);
  if (open && !present) setPresent(true);
  return present ? (
    <OpenDialog
      {...rest}
      leaving={!open}
      onGone={() => {
        setPresent(false);
      }}
    />
  ) : null;
}

function OpenDialog({ onClose, labelledBy, label, className, children, leaving, onGone }: Omit<DialogProps, 'open'> & { leaving: boolean; onGone: () => void }) {
  const panel = useRef<HTMLDivElement>(null);

  useKey('sheet', ['Escape'], onClose, () => !leaving);
  // The parts behind the veil get their keys back as soon as it starts to leave.
  useEffect(() => (leaving ? undefined : stopKeysAt('sheet')), [leaving]);

  // Where focus was when the dialog opened, to go back to, once.
  const cameFrom = useRef<Element | null>(null);
  const returnFocus = () => {
    const before = cameFrom.current;
    cameFrom.current = null;
    if (before instanceof HTMLElement && before.isConnected) before.focus({ preventScroll: true });
  };

  // Layout effects, so focus has moved before the browser paints.
  useLayoutEffect(() => {
    const sheet = panel.current;
    if (sheet === null) return;
    if (leaving) {
      // Here and not in the clean-up below: React puts focus back where it
      // was after any clean-up that moves it while the sheet is still mounted.
      returnFocus();
      return;
    }
    cameFrom.current = document.activeElement;
    const first = sheet.querySelector<HTMLElement>('[data-autofocus]') ?? stopsIn(sheet)[0] ?? sheet;
    first.focus({ preventScroll: true });
    let last = first;

    const keepFocusInside = (event: FocusEvent) => {
      const at = event.target;
      if (!(at instanceof HTMLElement)) return;
      if (sheet.contains(at)) {
        last = at;
        return;
      }
      if (at.closest('[role="dialog"]') !== null) return;
      (last.isConnected ? last : sheet).focus({ preventScroll: true });
    };
    document.addEventListener('focusin', keepFocusInside);
    return () => {
      document.removeEventListener('focusin', keepFocusInside);
    };
  }, [leaving]);
  // Taken off the page while still open: focus goes back all the same.
  useLayoutEffect(() => returnFocus, []);

  // Tab goes round inside the sheet.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'Tab' || event.metaKey || event.ctrlKey || event.altKey) return;
    const stops = stopsIn(event.currentTarget);
    const from = document.activeElement;
    const edge = event.shiftKey ? stops[0] : stops.at(-1);
    const next = event.shiftKey ? stops.at(-1) : stops[0];
    if (from !== edge && from !== event.currentTarget && stops.length > 0) return;
    event.preventDefault();
    (next ?? event.currentTarget).focus({ preventScroll: true });
  };

  const onAnimationEnd = (event: AnimationEvent<HTMLDivElement>) => {
    if (leaving && event.target === event.currentTarget) onGone();
  };

  return (
    <>
      <div
        className="dialog-veil"
        data-leaving={leaving || undefined}
        onMouseDown={(event) => {
          // A press on the veil does nothing, and does not take focus out of the sheet.
          event.preventDefault();
        }}
      />
      <div
        ref={panel}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        aria-label={label}
        tabIndex={-1}
        inert={leaving}
        className={className === undefined ? 'dialog' : `dialog ${className}`}
        data-leaving={leaving || undefined}
        onKeyDown={onKeyDown}
        onAnimationEnd={onAnimationEnd}
      >
        {children}
      </div>
    </>
  );
}
