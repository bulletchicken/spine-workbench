import type { RowState } from '@/look-and-feel/data-attributes';
import { StateIcon } from '@/look-and-feel/StateIcon';
import './CardRows.css';

/** One row of a card: the sign of its state, a title, and the lines under it. */
export interface CardRow {
  /** Tells this row from the others in its card, and follows it when rows move. */
  id: string;
  state: RowState;
  title: string;
  /** Each on a line of its own under the title. */
  lines?: readonly string[] | undefined;
}

interface CardRowsProps {
  rows: readonly CardRow[];
  /** The rows to flash once, by id. */
  flashing?: ReadonlySet<string> | undefined;
  /** A row's flash has run. */
  onFlashed?: ((id: string) => void) | undefined;
}

/**
 * The rows of a card, with lighter rules between them. A row's state
 * reaches CSS as `data-state`; its sign is decoration, and the title and
 * lines say the same in words.
 */
export function CardRows({ rows, flashing, onFlashed }: CardRowsProps) {
  return (
    <ul className="card-rows">
      {rows.map((row) => (
        <li
          key={row.id}
          className="card-row"
          data-state={row.state}
          data-flash={flashing?.has(row.id) === true || undefined}
          onAnimationEnd={
            onFlashed === undefined
              ? undefined
              : () => {
                  onFlashed(row.id);
                }
          }
        >
          <StateIcon state={row.state} />
          <div>
            <b className="card-row-title">{row.title}</b>
            {row.lines?.map((line) => (
              <span key={line} className="card-row-line">
                {line}
              </span>
            ))}
          </div>
        </li>
      ))}
    </ul>
  );
}
