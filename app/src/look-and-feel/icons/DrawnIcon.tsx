import type { ReactNode } from 'react';
import './icon.css';
import type { DrawnIconName } from './icon-name';

/** What every icon takes. */
export interface IconProps {
  className?: string | undefined;
}

interface DrawnIconProps extends IconProps {
  name: DrawnIconName;
  children: ReactNode;
}

/**
 * The frame the drawn icons share: a 20 px grid, a line with round ends,
 * the colour of the text around it. Decoration only: the control an icon
 * sits in carries the words.
 */
export function DrawnIcon({ name, className, children }: DrawnIconProps) {
  return (
    <svg
      className={className === undefined ? 'icon' : `icon ${className}`}
      data-icon={name}
      viewBox="0 0 20 20"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  );
}
