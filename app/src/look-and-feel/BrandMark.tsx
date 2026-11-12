interface BrandMarkProps {
  className?: string | undefined;
}

/** Three vertebrae, beside the product's name. Decoration only. */
export function BrandMark({ className }: BrandMarkProps) {
  return (
    <svg className={className === undefined ? 'brand-mark' : `brand-mark ${className}`} viewBox="0 0 20 20" aria-hidden="true" focusable="false">
      <rect x="6" y="2.75" width="9" height="3.5" rx="1.25" />
      <rect x="4.5" y="8.25" width="9" height="3.5" rx="1.25" />
      <rect x="6" y="13.75" width="9" height="3.5" rx="1.25" />
    </svg>
  );
}
