import { DrawnIcon, type IconProps } from './DrawnIcon';

/** Pins: the pointer that picks up a corner and drags it. */
export function PinIcon({ className }: IconProps) {
  return (
    <DrawnIcon name="pin" className={className}>
      <path d="M3.9 3.9l5.4 12.2 1.7-5.1 5.1-1.7z" />
    </DrawnIcon>
  );
}
