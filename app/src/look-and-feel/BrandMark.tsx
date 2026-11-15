import { useId } from 'react';

interface BrandMarkProps {
  className?: string | undefined;
}

/**
 * Three vertebrae, beside the product's name. Decoration only. The middle one, out of line, is
 * the melon light: filled and drawn with the mark's own gradient, whose stops the design system
 * colours. Each mark drawn needs its own gradient id, so the gallery can draw several.
 */
export function BrandMark({ className }: BrandMarkProps) {
  const melon = `${useId()}-melon`;
  return (
    <svg className={className === undefined ? 'brand-mark' : `brand-mark ${className}`} viewBox="0 0 20 20" aria-hidden="true" focusable="false">
      <linearGradient id={melon}>
        <stop className="melon-1" offset="0" />
        <stop className="melon-2" offset="1" />
      </linearGradient>
      <rect x="6" y="2.75" width="9" height="3.5" rx="1.25" />
      <rect x="4.5" y="8.25" width="9" height="3.5" rx="1.25" fill={`url(#${melon})`} stroke={`url(#${melon})`} />
      <rect x="6" y="13.75" width="9" height="3.5" rx="1.25" />
    </svg>
  );
}
