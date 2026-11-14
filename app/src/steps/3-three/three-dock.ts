import { TEMPLATE_LEVELS } from '@/shapes';
import type { DockSpec } from '@/guide/dock-spec';
import type { SpineFit } from '@/shapes';
import type { ThreeView } from '@/view-state/three-state';
import { THREE_WORDS as W } from '@/words/three';
export function threeDock(fit: SpineFit, suppliedSide: boolean, chosen: ThreeView | null, choose: (name: ThreeView) => void): DockSpec {
  const levels = new Set(fit.vertebrae.filter((v) => v.level !== 'C7' && TEMPLATE_LEVELS.standard.includes(v.level)).map((v) => v.level));
  const words = [fit.sideFilm === null ? W.front : W.pair];
  if (suppliedSide && fit.sideFilm === null) words.push(W.unusedSide);
  if (levels.size < 17) words.push(W.partial);
  return { scene: 'three', sentence: W.settled, helper: { kind: 'words', words: [words.join(' ')] }, views: { chosen, choose } };
}
