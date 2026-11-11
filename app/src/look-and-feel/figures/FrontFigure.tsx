import { FigureSpine, ViewFigure, type ViewFigureProps } from './ViewFigure';

/** Standing, seen from the front: square shoulders, two legs, a straight spine. */
export function FrontFigure({ className }: ViewFigureProps) {
  return (
    <ViewFigure view="front" className={className}>
      <circle cx="30" cy="12.5" r="8" />
      <path d="M18 112V83C18 78.6 14.7 79.4 14.7 75L13.5 35A9 9 0 0 1 22.5 26H37.5A9 9 0 0 1 46.5 35L45.3 75C45.3 79.4 42 78.6 42 83V112" />
      <path d="M30 94V112" />
      <FigureSpine d="M30 31V79" />
    </ViewFigure>
  );
}
