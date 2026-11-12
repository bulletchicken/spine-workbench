import type { CSSProperties } from 'react';

interface SegmentedOption {
  id: string;
  label: string;
  /** Cannot be pressed, and is left out of the tab order. */
  disabled?: boolean;
}

interface SegmentedProps {
  /** Its spoken name. */
  label: string;
  options: readonly SegmentedOption[];
  /** The option the thumb sits under, or none. */
  chosen: string | null;
  onChoose: (id: string) => void;
  /** Which control it is: the step control passes `steps`. */
  className?: string;
}

/**
 * A pill of options with a white thumb that slides under the chosen one: the steps of the
 * journey. It is a list of places to go, so it is a `nav` and the chosen one is the current one.
 * Every rule is the design system's `.seg`. That is a block grid, as narrow as its options only
 * inside a column sized to its content, which the bar gives it.
 */
export function Segmented({ label, options, chosen, onChoose, className }: SegmentedProps) {
  const at = options.findIndex((option) => option.id === chosen);
  return (
    <nav
      className={className === undefined ? 'seg' : `seg ${className}`}
      aria-label={label}
      data-none={at < 0 ? '' : undefined}
      // Where the thumb sits is geometry, so it is a number written on the element.
      style={{ '--at': Math.max(at, 0) } as CSSProperties}
    >
      {options.map((option) => (
        <button
          key={option.id}
          type="button"
          disabled={option.disabled}
          aria-current={option.id === chosen ? 'true' : undefined}
          onClick={() => {
            onChoose(option.id);
          }}
        >
          {option.label}
        </button>
      ))}
    </nav>
  );
}
