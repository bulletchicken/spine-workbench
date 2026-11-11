import type { ReactNode } from 'react';
import type { FilmKind } from '@/shapes';
import './view-figure.css';

/** What every view figure takes. */
export interface ViewFigureProps {
  className?: string | undefined;
}

interface ViewFigureFrameProps extends ViewFigureProps {
  view: FilmKind;
  children: ReactNode;
}

/**
 * The frame the four view figures share: a 60 by 120 drawing in the colour
 * of the text around it. Decoration only: the slot it stands in says which
 * view it is in words.
 */
export function ViewFigure({ view, className, children }: ViewFigureFrameProps) {
  return (
    <svg
      className={className === undefined ? 'view-figure' : `view-figure ${className}`}
      data-view={view}
      viewBox="0 0 60 120"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  );
}

interface SpineProps {
  d: string;
}

/**
 * A figure's spine. Its length is counted as 45 whatever it measures, so
 * the dashes come out whole: seven of them, ending on a dash.
 */
export function FigureSpine({ d }: SpineProps) {
  return <path className="view-figure-spine" pathLength="45" d={d} />;
}
