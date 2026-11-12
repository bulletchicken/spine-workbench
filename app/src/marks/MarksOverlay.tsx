import type { FilmLayerProps } from '@/film-viewer/film-space';
import type { MarkToDraw, Quad } from './mark-to-draw';
import './marks.css';

export interface MarksOverlayProps {
  /** For a screen reader: "Outlines on the Front film". */
  label: string;
  /** Every vertebra to draw, top to bottom. */
  marks: readonly MarkToDraw[];
  /** The key of the outline pointed at. */
  on: string | null;
  /** In a review: the key of the one outline in focus; the rest stand back. */
  only: string | null;
  /** The pointer came onto a shown outline (its key) or left it (`null`). */
  onPoint?: (key: string | null) => void;
}

const pointsOf = (quad: Quad) => quad.map((corner) => `${String(corner[0])},${String(corner[1])}`).join(' ');
const rightOf = (quad: Quad) => Math.max(...quad.map((corner) => corner[0]));
const middleOf = (quad: Quad) => (quad[0][1] + quad[1][1] + quad[2][1] + quad[3][1]) / 4;

/**
 * Every outline of one film, drawn as the design system's stylesheet lays it
 * out, in film pixels: first every vertebra's fill and casing, then every
 * vertebra's line and name, so that no casing darkens a neighbour's line.
 * The two groups of a vertebra carry the same state classes, and nothing here
 * names a colour, a width or a time. The name is always drawn; its tile's
 * `data-names` hides it on the small films.
 *
 * Only a shown outline answers the pointer, and none while a review is on.
 * Nothing here takes focus or a press: the review walks the outlines that
 * matter by name. It assumes the film is shown as stored, not turned.
 *
 * A FilmLayer part: `useLayer('outlines', MarksOverlay, { label, marks, on, only, onPoint })`.
 */
export function MarksOverlay({ label, marks, on, only, onPoint }: MarksOverlayProps & FilmLayerProps) {
  const classes = (base: string, mark: MarkToDraw) =>
    [base, mark.shown && 'found', mark.look === 'look' && 'look', on === mark.key && 'on', only === mark.key && 'focus'].filter((each) => each !== false).join(' ');
  const answers = (mark: MarkToDraw) => onPoint !== undefined && mark.shown && only === null;
  return (
    <g className="marks" role="group" aria-label={label}>
      {marks.map((mark) => (
        <g key={`under ${mark.key}`} className={classes('mark under', mark)} aria-hidden="true">
          <polygon points={pointsOf(mark.quad)} pathLength={1} />
        </g>
      ))}
      {marks.map((mark) => (
        <g key={mark.key} className={classes('mark', mark)} role="group" aria-label={mark.spoken} aria-hidden={mark.shown ? undefined : true}>
          <polygon
            points={pointsOf(mark.quad)}
            pathLength={1}
            onPointerEnter={
              answers(mark)
                ? () => {
                    onPoint?.(mark.key);
                  }
                : undefined
            }
            onPointerLeave={
              answers(mark)
                ? () => {
                    onPoint?.(null);
                  }
                : undefined
            }
          />
          <text x={rightOf(mark.quad)} y={middleOf(mark.quad)}>
            {mark.name}
          </text>
        </g>
      ))}
    </g>
  );
}
