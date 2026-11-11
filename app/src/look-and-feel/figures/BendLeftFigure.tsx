import { FigureSpine, ViewFigure, type ViewFigureProps } from './ViewFigure';

/**
 * The front figure with everything above the hips bent as one arc, 24
 * degrees at the head. As drawn it leans to the left of the picture.
 */
export function BendDrawing() {
  return (
    <>
      <circle cx="20.3" cy="14.4" r="8" />
      <path d="M22 112V83C22 78.6 18.7 79.4 18.7 75.4C17.4 63.4 15.2 51.6 12.1 40.1C10.8 35.8 13.5 31.1 18.1 29.4L32.3 24.5C37 22.9 42.2 25.8 43.8 31C46.8 45.4 48.7 60 49.2 74.6C49.3 79.4 46 78.6 46 83V112" />
      <path d="M34 94V112" />
      <FigureSpine d="M26.8 31.7C27.9 35.6 29 39.4 29.9 43.3C30.8 47.2 31.5 51.1 32.2 55.1C32.7 59.1 33.2 63 33.5 67C33.8 71 33.9 75 34 79" />
    </>
  );
}

/** Leaning to the left. */
export function BendLeftFigure({ className }: ViewFigureProps) {
  return (
    <ViewFigure view="bend-left" className={className}>
      <BendDrawing />
    </ViewFigure>
  );
}
