import type { Spot } from '../stage-layout';
export function comparisonLayout(width: number, height: number, reserve: number, gap: number, clear: number, maxHeight: number, caption: number) {
  const filmCentre = (reserve + width / 2) / 2;
  const spineCentre = width - filmCentre;
  const available = width / 2 - reserve - 2 * clear;
  const tall = Math.max(0, Math.min(height - caption, maxHeight, available * 2));
  const film: Spot = { x: filmCentre - tall / 4, y: (height - tall) / 2, width: tall / 2, height: tall, radius: 'large' };
  return { film, shift: spineCentre - filmCentre, reserve: reserve > 0 ? reserve + gap : 0 };
}
