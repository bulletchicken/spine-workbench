import './BrandMark.css';

interface BrandMarkProps {
  className?: string | undefined;
}

/**
 * The app's mark: three vertebrae, each a little larger and brighter than
 * the one above, leaning as a spine does. Decoration only: the app's name
 * stands beside it.
 */
export function BrandMark({ className }: BrandMarkProps) {
  return (
    <svg
      className={className === undefined ? 'brand-mark' : `brand-mark ${className}`}
      viewBox="0 0 26 26"
      aria-hidden="true"
      focusable="false"
    >
      <rect className="brand-mark-plate" x="0.5" y="0.5" width="25" height="25" rx="7" />
      <rect className="brand-mark-body" x="9.2" y="4.9" width="8" height="3.7" rx="1.3" opacity="0.5" transform="rotate(-7 13.2 6.75)" />
      <rect className="brand-mark-body" x="8.4" y="11.1" width="9.2" height="3.9" rx="1.3" opacity="0.76" />
      <rect className="brand-mark-body" x="8.5" y="17.4" width="9.8" height="4.1" rx="1.3" transform="rotate(7 13.4 19.45)" />
    </svg>
  );
}
