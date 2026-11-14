import { namedView } from '@/spine-3d/looking/framing';
import type { ViewName } from '@/spine-3d/looking/named-views';
import type { Stage } from '@/spine-3d/scene/stage';

/** Reserve the same clear space on both sides, keeping the spine centred. */
export function restingView(stage: Stage, part: Element, name: ViewName = 'three-quarter') {
  const css = getComputedStyle(part);
  const read = (key: string) => parseFloat(css.getPropertyValue(key)) || 0;
  const narrow = window.innerWidth <= (read('--break-narrow-w') || 900);
  const reserve = (narrow ? read('--rail-h') / 2 : read('--panel-w')) + read('--tile-gap');
  const size = stage.size();
  const view = namedView(stage.bounds(), { width: Math.max(1, size.width - 2 * reserve), height: size.height }, name);
  return { ...view, aimAt: [size.width / 2, size.height / 2] as const };
}
