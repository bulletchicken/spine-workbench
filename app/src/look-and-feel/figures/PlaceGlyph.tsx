import type { FilmKind } from '@/shapes';

const curves: Record<FilmKind, (t: number) => number> = {
  front: (t) => 4 * Math.sin(2 * Math.PI * t),
  side: (t) => 4 - 9 * Math.sin(Math.PI * t),
  'bend-left': (t) => 5 - 15 * (1 - t) ** 2,
  'bend-right': (t) => 15 * (1 - t) ** 2 - 5,
};

export function PlaceGlyph({ kind, className }: { kind: FilmKind; className?: string }) {
  const curve = curves[kind];
  return (
    <svg viewBox="0 0 40 73" aria-hidden="true" className={className}>
      {Array.from({ length: 7 }, (_, index) => {
        const t = index / 6;
        const x = 20 + curve(t);
        const y = 8 + 9.5 * index;
        const angle = -Math.atan2(curve(t + 0.05) - curve(t - 0.05), 5.7) * 180 / Math.PI;
        return <rect key={index} x={x - 6} y={y - 2.75} width={12} height={5.5} rx={1.75} transform={`rotate(${String(angle)} ${String(x)} ${String(y)})`} />;
      })}
    </svg>
  );
}
