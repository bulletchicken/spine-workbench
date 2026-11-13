import './HalfFilmFigure.css';

interface HalfFilmFigureProps {
  /** The half of the picture this choice means. */
  half: 'left' | 'right';
  /** What stands in that half: the letter of the side it would be, such as L. */
  letter: string;
}

/**
 * A film in small, with one half picked out: the figure of a choice that
 * means "this side of the picture". It is black, as every film is.
 * Decoration only: the card says the same in words.
 */
export function HalfFilmFigure({ half, letter }: HalfFilmFigureProps) {
  return (
    <span className="half-film-figure" data-ground="film" data-half={half} aria-hidden="true">
      <i>{half === 'left' ? letter : null}</i>
      <i>{half === 'right' ? letter : null}</i>
    </span>
  );
}
