import { BendDrawing } from './BendLeftFigure';
import { ViewFigure, type ViewFigureProps } from './ViewFigure';

/** Leaning to the right: the left-leaning figure, mirrored. */
export function BendRightFigure({ className }: ViewFigureProps) {
  return (
    <ViewFigure view="bend-right" className={className}>
      <g transform="matrix(-1 0 0 1 60 0)">
        <BendDrawing />
      </g>
    </ViewFigure>
  );
}
