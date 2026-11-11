import { FigureSpine, ViewFigure, type ViewFigureProps } from './ViewFigure';

/**
 * Standing, seen from the side and facing right: a nose, a narrower body
 * with one leg showing, and the spine's own curves, out at the upper back
 * and in at the waist.
 */
export function SideFigure({ className }: ViewFigureProps) {
  return (
    <ViewFigure view="side" className={className}>
      <path d="M38.8 16.6A8 8 0 1 1 39.9 12.1L41.5 14.9Z" />
      <path d="M24.5 112V84C24.5 79 20.4 79.4 20.4 75C20.4 70 22.8 67 22.8 62C22.8 55 20 50 20 42V35A9.5 9 0 0 1 29.5 26H32A9 9 0 0 1 41 35V46C41 56 40 62 40 75C40 79.4 37 78.6 37 83V112" />
      <FigureSpine d="M27.2 31C26.8 32 25.5 34.8 25 37C24.5 39.2 24.3 41.5 24.4 44C24.5 46.5 25.1 49.3 25.6 52C26.1 54.7 26.8 57.3 27.2 60C27.6 62.7 27.8 65.7 27.8 68C27.8 70.3 27.4 72.2 27 74C26.6 75.8 25.8 78.2 25.6 79" />
    </ViewFigure>
  );
}
