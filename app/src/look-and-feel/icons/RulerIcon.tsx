import { DrawnIcon, type IconProps } from './DrawnIcon';

/** The ruler: a rule laid across the film, with its ticks. */
export function RulerIcon({ className }: IconProps) {
  return (
    <DrawnIcon name="ruler" className={className}>
      <path d="M2.9 13.3L13.3 2.9l3.8 3.8L6.7 17.1z" />
      <path className="icon-fine" d="M5.5 10.7l1.3 1.3M8.1 8.1l1.8 1.8M10.7 5.5l1.3 1.3" />
    </DrawnIcon>
  );
}
