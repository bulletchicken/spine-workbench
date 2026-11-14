import { filmSpace } from '@/film-viewer/film-space';
import type { FilmCamera, Px } from '@/spine-3d/placing/film-camera';
import { fromFilm } from '@/spine-3d/placing/film-camera';
import { eyeDistance } from '@/spine-3d/looking/lens';
import { type View, type CanvasSize, viewAxes } from '@/spine-3d/looking/view';
import { toPatient, toScene } from '@/spine-3d/placing/scene-axes';
import { type Vec3, add, dot, mix, scale, sub } from '@/spine-3d/placing/vectors';
import type { MoveFilm } from '../../Stage';

const FILM_SINK = 380;

export function cropPixels(frame: MoveFilm['opening']): Px[] {
  const space = filmSpace(frame.placement);
  return [[0, 0], [frame.width, 0], [frame.width, frame.height], [0, frame.height]].map(([x = 0, y = 0]) => space.toFilm([x, y]));
}
export function cropWall(camera: FilmCamera, pixels: readonly Px[], out: number): Vec3[] {
  return pixels.map((pixel) => fromFilm(camera, pixel, out));
}
/** Inverse projection onto a plane parallel to the screen at a known depth. */
export function screenPoint(view: View, size: CanvasSize, pixel: Px, depth: number): Vec3 {
  const axes = viewAxes(view.azimuth, view.elevation), distance = eyeDistance(view, size);
  const grown = distance === null ? 1 : (distance - depth) / distance;
  return toPatient(add(add(add(view.aim, scale(axes.right, (pixel[0] - view.aimAt[0]) / view.pxPerMm[0] * grown)), scale(axes.up, -(pixel[1] - view.aimAt[1]) / view.pxPerMm[1] * grown)), scale(axes.toViewer, depth)));
}
export function flyWall(corners: readonly Vec3[], rail: MoveFilm['rail'], view: View, size: CanvasSize, progress: number): Vec3[] {
  const axes = viewAxes(view.azimuth, view.elevation);
  const near = corners.reduce((sum, point) => sum + dot(sub(toScene(point), view.aim), axes.toViewer), 0) / Math.max(1, corners.length);
  return corners.map((point, i) => {
    const x = i === 1 || i === 2 ? rail.width : 0, y = i >= 2 ? rail.height : 0;
    return mix(point, screenPoint(view, size, [rail.x + x, rail.y + y], near - FILM_SINK), progress);
  });
}
