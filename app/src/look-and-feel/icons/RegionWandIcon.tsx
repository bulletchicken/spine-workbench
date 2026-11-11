import { DrawnIcon, type IconProps } from './DrawnIcon';

/** The region wand: a wand whose tip throws sparks. */
export function RegionWandIcon({ className }: IconProps) {
  return (
    <DrawnIcon name="region-wand" className={className}>
      <path d="M3.4 16.6l8.5-8.5" />
      <path className="icon-fine" d="M13.5 4.3V1.9M15.7 6.5h2.4M13.5 8.7v1.6M11.3 6.5H9.7M15.1 4.9l1-1M11.9 4.9l-1-1M15.1 8.1l1 1" />
    </DrawnIcon>
  );
}
