import { DrawnIcon, type IconProps } from './DrawnIcon';

/** The magnifier. */
export function MagnifierIcon({ className }: IconProps) {
  return (
    <DrawnIcon name="magnifier" className={className}>
      <circle cx="8.7" cy="8.7" r="5.3" />
      <path d="M12.6 12.6l4.3 4.3" />
    </DrawnIcon>
  );
}
