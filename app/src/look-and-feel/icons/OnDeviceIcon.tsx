import { DrawnIcon, type IconProps } from './DrawnIcon';

/** On this device only: a house with a tick inside. */
export function OnDeviceIcon({ className }: IconProps) {
  return (
    <DrawnIcon name="on-device" className={className}>
      <path d="M3.3 9.1L10 3.5l6.7 5.6v6.3a1.2 1.2 0 0 1-1.2 1.2h-11a1.2 1.2 0 0 1-1.2-1.2z" />
      <path d="M7.3 11.5l1.9 1.9 3.5-3.8" />
    </DrawnIcon>
  );
}
