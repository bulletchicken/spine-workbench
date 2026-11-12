import type { Px } from '@/shapes';
import { fitZoom, LOOK_HEIGHT, type CameraFrame } from '@/film-viewer/camera/camera';

export interface Box { left: number; top: number; right: number; bottom: number }

export type FilmFrame =
  | { kind: 'whole' }
  | { kind: 'spine'; box: Box; names: boolean; most: number }
  | { kind: 'vertebra'; centre: Px; names: boolean; over: number };

/** Clear space on each side of the spine, in screen pixels. */
export const SPINE_ROOM = 24;

export function spineFraming(camera: CameraFrame, box: Box, nameRoom: number, most: number): { point: Px; over: number } {
  const fit = fitZoom(camera);
  const zoom = Math.max(fit, Math.min(most * fit, (camera.viewport.width - 2 * SPINE_ROOM) / (box.right - box.left + 2 * nameRoom), (camera.viewport.height - 2 * SPINE_ROOM) / (box.bottom - box.top)));
  return {
    point: [(box.left + box.right) / 2 + nameRoom, (box.top + box.bottom) / 2 - (0.5 - LOOK_HEIGHT) * camera.viewport.height / zoom],
    over: zoom / fit,
  };
}

export function vertebraFraming(camera: CameraFrame, centre: Px, nameRoom: number, over: number): { point: Px; over: number } {
  return { point: [centre[0] + nameRoom, centre[1] - (0.5 - LOOK_HEIGHT) * camera.viewport.height / (over * fitZoom(camera))], over };
}
