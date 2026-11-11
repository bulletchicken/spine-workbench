import { cloneElement, useId, useState, type ReactElement, type ReactNode } from 'react';
import { useKey } from './keyboard/use-key';
import './Tooltip.css';

interface TooltipProps {
  /** What it says. */
  tip: ReactNode;
  /** `name` is one short line; `help` is a sentence or two that wraps, and may open with a `<b>` title. */
  look?: 'name' | 'help';
  /** Which side of the control it opens on. */
  side?: 'above' | 'below' | 'left' | 'right';
  /** Where it sits along that side. `end` keeps a tooltip at the window's right edge inside the window. */
  align?: 'start' | 'centre' | 'end';
  /** `false` when it only repeats the control's own name: it is then shown and not spoken twice. */
  spoken?: boolean;
  /** The one control it is about. */
  children: ReactElement<{ 'aria-describedby'?: string | undefined }>;
}

/**
 * A few words beside a control, shown while the pointer rests on it or
 * the keyboard's focus is on it. A press puts it away, and so does
 * Escape, which is still passed on to whatever else answers it. It never
 * takes a press itself, so nothing under it is blocked.
 */
export function Tooltip({ tip, look = 'name', side = 'below', align = 'centre', spoken = true, children }: TooltipProps) {
  const id = useId();
  const [shown, setShown] = useState(false);
  const putAway = () => {
    setShown(false);
  };

  useKey(
    'always',
    ['Escape'],
    () => {
      putAway();
      return false;
    },
    () => shown,
  );

  return (
    <span
      className="tooltip-anchor"
      onPointerEnter={() => {
        setShown(true);
      }}
      onPointerLeave={putAway}
      onPointerDown={putAway}
      onFocus={(event) => {
        if (event.target.matches(':focus-visible')) setShown(true);
      }}
      onBlur={putAway}
    >
      {spoken ? cloneElement(children, { 'aria-describedby': id }) : children}
      <span
        role="tooltip"
        id={id}
        className="tooltip"
        data-look={look}
        data-side={side}
        data-align={align}
        data-shown={shown || undefined}
        aria-hidden={spoken ? undefined : true}
      >
        {tip}
      </span>
    </span>
  );
}
