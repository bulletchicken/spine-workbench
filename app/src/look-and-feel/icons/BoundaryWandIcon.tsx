import { DrawnIcon, type IconProps } from './DrawnIcon';

/** The boundary wand: two vertebrae and the broken line of the gap between them. */
export function BoundaryWandIcon({ className }: IconProps) {
  return (
    <DrawnIcon name="boundary-wand" className={className}>
      <rect x="4.8" y="2.6" width="10.4" height="4.7" rx="1.5" />
      <rect x="4.8" y="12.7" width="10.4" height="4.7" rx="1.5" />
      <path d="M1.9 10h1.5M6.8 10h1.5M11.7 10h1.5M16.6 10h1.5" />
    </DrawnIcon>
  );
}
