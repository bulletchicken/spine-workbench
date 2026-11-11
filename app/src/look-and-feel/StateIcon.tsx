import type { ReactNode } from 'react';
import type { RowState } from './data-attributes';
import './StateIcon.css';

// The sign for the state of a fact chip, a check row or a card row. Six
// signs for the seven states: a chip that `needs` an answer wears the same
// warning as a row that warns. All are drawn on the icons' 20 px grid.

const RING = <circle cx="10" cy="10" r="7.2" />;
const WARNING = (
  <>
    <path d="M10 3.3l7.3 12.8H2.7z" />
    <path d="M10 8.4v3.4M10 14.2v.1" />
  </>
);

const SIGNS: Record<RowState, ReactNode> = {
  // Done, or true: a tick in a ring.
  ok: (
    <>
      {RING}
      <path d="M6.7 10.3l2.3 2.3 4.4-4.8" />
    </>
  ),
  // Not yet: a ring of dots.
  wait: <circle className="state-icon-dots" cx="10" cy="10" r="7.2" pathLength="12" />,
  // Working on it: an arc that turns on a faint ring.
  busy: (
    <>
      <circle className="state-icon-track" cx="10" cy="10" r="7.2" />
      <path d="M10 2.8a7.2 7.2 0 0 1 7.2 7.2" />
    </>
  ),
  // Set by a person: a head and shoulders in a ring.
  you: (
    <>
      {RING}
      <circle className="state-icon-solid" cx="10" cy="8" r="2.2" />
      <path d="M6 14.3c.9-1.8 2.3-2.7 4-2.7s3.1.9 4 2.7" />
    </>
  ),
  // Worth knowing, or waiting for an answer: a warning triangle.
  warn: WARNING,
  needs: WARNING,
  // Wrong, or failed: a cross in a ring.
  bad: (
    <>
      {RING}
      <path d="M7.4 7.4l5.2 5.2M12.6 7.4l-5.2 5.2" />
    </>
  ),
};

interface StateIconProps {
  state: RowState;
  className?: string | undefined;
}

/**
 * The sign for one state, in that state's colour. Decoration only: the row
 * or chip it sits in says the state in words.
 */
export function StateIcon({ state, className }: StateIconProps) {
  return (
    <svg
      className={className === undefined ? 'state-icon' : `state-icon ${className}`}
      data-state={state}
      viewBox="0 0 20 20"
      aria-hidden="true"
      focusable="false"
    >
      {SIGNS[state]}
    </svg>
  );
}
