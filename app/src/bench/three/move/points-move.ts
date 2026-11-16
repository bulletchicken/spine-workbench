import type { Stage, FilmName } from '@/spine-3d/scene/stage';
import type { DrawnSpine } from '@/spine-3d/scene/drawn-spine';
import type { View } from '@/spine-3d/looking/view';
import { azimuthAlong, shortWayRound } from '@/spine-3d/looking/named-views';
import { viewBetween } from '@/spine-3d/motion/easing';
import { toCanvas } from '@/spine-3d/looking/lens';
import { toScene } from '@/spine-3d/placing/scene-axes';
import { type Vec3, add, dot, mix, scale } from '@/spine-3d/placing/vectors';
import { filmPlane } from '@/spine-3d/placing/film-plane';
import type { FilmWall, FilmWallValues } from '@/spine-3d/scene/film-wall';
import type { FilmDriver, MoveFilm } from '../../Stage';
import { cropPixels, cropWall, flyWall, screenPoint } from './film-flight';
import { sample, scriptFor } from './points-timeline';

const ROOM = { pairedAzimuth: 42, singleAzimuth: 36, elevation: 12, scale: .95, perspective: .6, drift: 6, drop: 4, singleLead: .42, behind: 30, corner: 1.5 } as const;

export function pointsMove(stage: Stage, spine: DrawnSpine, films: FilmDriver, canvas: HTMLCanvasElement, from: View, to: View) {
  const box = canvas.getBoundingClientRect();
  const walls: { name: FilmName; wall: FilmWall; source: MoveFilm; pixels: ReturnType<typeof cropPixels>; railPixels: ReturnType<typeof cropPixels>; corners: Vec3[]; out: number; saved: FilmWallValues }[] = [];
  for (const name of ['front', 'side'] as const) {
    const wall = stage.film(name), source = films.moveFilm?.(name, box);
    if (wall == null || source == null) continue;
    walls.push({ name, wall, source, pixels: cropPixels(source.opening), railPixels: cropPixels(source.rail), corners: [], out: wall.plane().out, saved: wall.values() });
  }
  const front = walls.find(({ name }) => name === 'front'), side = walls.find(({ name }) => name === 'side');
  const bounds = stage.bounds();
  const back = front?.wall.plane().towardReader;
  if (front !== undefined && back !== undefined) {
    // A single wall stands thirty millimetres behind the fitted bones.
    front.out = Math.min(...[bounds.low[2], bounds.high[2]].map((x) => x * back[0])) - ROOM.behind;
    if (side !== undefined) {
      const sideNormal = side.wall.plane().towardReader;
      const frontCrop = cropWall(front.wall.camera, front.pixels, 0), sideCrop = cropWall(side.wall.camera, side.pixels, 0);
      front.out = Math.min(front.out, ...sideCrop.map((point) => dot(point, back)));
      side.out = Math.min(...frontCrop.map((point) => dot(point, sideNormal)));
    }
  }
  for (const entry of walls) {
    entry.corners = cropWall(entry.wall.camera, entry.pixels, entry.out);
    entry.wall.show({ data: entry.source.film.pixels, width: entry.source.film.size.width, height: entry.source.film.size.height });
    entry.wall.set({ out: entry.out });
  }
  for (const level of spine.levels) spine.formation.script(level, scriptFor(level));
  // Front alone: each bone forms flat on its outline, then rises square out of the film to the fit.
  // With Side as well a bone cannot lie on both films; it forms where their threads cross.
  spine.formation.press(front !== undefined && side === undefined ? filmPlane(front.wall.camera, front.out) : null);
  spine.formation.threads(walls.map(({ wall, out }) => filmPlane(wall.camera, out)));
  const frontCentre = front === undefined ? from.aim : toScene(front.corners.reduce<Vec3>((sum, point) => add(sum, scale(point, .25)), [0, 0, 0]));
  const pivot = front === undefined || side === undefined ? frontCentre : toScene(add(add(scale(front.wall.plane().towardReader, front.out), scale(side.wall.plane().towardReader, side.out)), [0, 0, frontCentre[1]]));
  const direction = side === undefined ? -1 : Math.sign(shortWayRound(from.azimuth, azimuthAlong(side.wall.camera.imageRight)) - from.azimuth);
  const book: View = { ...from, aim: mix(pivot, to.aim, side === undefined ? ROOM.singleLead : 0), azimuth: from.azimuth + direction * (side === undefined ? ROOM.singleAzimuth : ROOM.pairedAzimuth), elevation: ROOM.elevation, perspective: ROOM.perspective, pxPerMm: [from.pxPerMm[0] * ROOM.scale, from.pxPerMm[1] * ROOM.scale] };
  // Keep the opening wall's centre fixed while the room gains depth.
  const saved = front === undefined ? null : { x: side === undefined ? front.source.opening.x + front.source.opening.width / 2 : (front.source.opening.x + front.source.opening.width + side.source.opening.x) / 2, y: front.source.opening.y + front.source.opening.height / 2 };
  const openingAnchor = saved === null ? { x: 0, y: 0 } : toCanvas(from, stage.size(), book.aim);
  return {
    set(progress: number) {
      const s = sample(progress), standing = viewBetween(from, { ...book, azimuth: book.azimuth - direction * ROOM.drift * s.drift }, s.stand);
      if (saved !== null) {
        stage.setView({ ...standing, aimAt: [0, 0] });
        const at = stage.toCanvas(book.aim);
        standing.aimAt = [openingAnchor.x + (saved.x - openingAnchor.x) * s.stand - at.x, openingAnchor.y + (saved.y + ROOM.drop - openingAnchor.y) * s.stand - at.y];
      }
      const view = viewBetween(standing, to, s.settle); stage.setView(view);
      spine.set({ pose: 1, reveal: 1, fill: 1, strength: 1, tint: 0, cage: 0 });
      spine.setSpine({ depth: 1, fill: 1, revealRise: 0, cageOnFilm: 0, shadow: s.shadow, offset: [0, 0, 0], turned: [0, 0], scale: 1 });
      for (const level of spine.levels) { const bone = s.bone(level); spine.set({ fill: bone.fill > 0 ? 1 : 0, edgeFill: bone.fill >= 1 ? -1 : bone.fill }, level); }
      spine.formation.time(s.t, s.ownParts);
      for (const entry of walls) {
        const { source, wall, name } = entry;
        let corners = entry.corners;
        if (name === 'side' && s.swing < 1) {
          // At zero the sheet occupies its Find rectangle; its hinge stays at the inner edge.
          const a = source.opening;
          const flat = [[a.x, a.y], [a.x + a.width, a.y], [a.x + a.width, a.y + a.height], [a.x, a.y + a.height]] as const;
          corners = corners.map((point, i) => {
            const start = screenPoint(from, stage.size(), flat[i] ?? [0, 0], 0);
            const angle = s.swing * Math.PI / 2;
            const hinge = entry.corners[0] ?? point;
            const target = [point[0] - hinge[0], point[1] - hinge[1], 0] as const;
            const near = mix(start, hinge, 1 - Math.cos(angle));
            return [near[0] + target[0] * Math.sin(angle), near[1] + target[1] * Math.sin(angle), point[2]] as Vec3;
          });
        }
        const travel = s.trips[name];
        wall.place(travel > 0 ? flyWall(corners, source.rail, view, stage.size(), travel) : corners, entry.pixels.map((pixel, i) => { const target = entry.railPixels[i] ?? pixel; return [pixel[0] + (target[0] - pixel[0]) * travel, pixel[1] + (target[1] - pixel[1]) * travel]; }));
        wall.set({ strength: s.ownParts ? 1 - s.hand : 0, hinge: 1, corner: ROOM.corner });
        wall.marks(source.film.outlines.filter((mark) => mark.shown).map((mark) => ({ quad: mark.quad, lift: s.bone(mark.name as import('@/shapes').Level).lift, needsLook: mark.look === 'look' })));
      }
      return { ...s, walls: walls.map(({ name }) => name) };
    },
    clear() {
      spine.formation.time(0, false); spine.formation.press(null); spine.set({ edgeFill: -1 });
      for (const { wall, saved } of walls) { wall.set(saved); wall.place(null); wall.marks([]); wall.show(null); }
    },
  };
}
