import type { Stage } from '@/shapes';

const WEIGHTS: readonly [Stage, number][] = [['evidence', 0.30], ['centre-line', 0.15], ['strip', 0.10], ['chain', 0.30], ['naming', 0.05], ['confidence', 0.10]];

export function searchShare(progress: { stage: Stage; fraction: number }, frontAlone: boolean): number {
  const fraction = Number.isFinite(progress.fraction) ? Math.max(0, Math.min(1, progress.fraction)) : 0;
  if (frontAlone) {
    switch (progress.stage) {
      case 'evidence': return 0.9 * fraction;
      case 'centre-line':
      case 'strip': return 0.9;
      case 'chain': return 0.93;
      case 'naming': return 0.96;
      case 'confidence': return 0.96 + 0.04 * fraction;
    }
  }
  let before = 0;
  for (const [stage, weight] of WEIGHTS) {
    if (stage === progress.stage) return Math.min(1, before + weight * fraction);
    before += weight;
  }
  return 1;
}
