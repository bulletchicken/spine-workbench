import { expect, it } from 'vitest';
import { Group, Mesh, Points, Triangle, Vector3 } from 'three';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { MADE_UP_CASE, FRONT_ONLY_CASE } from '../made-up-case/made-up-case';
import { fittedFilmCamera } from '../placing/fitted-film-camera';
import { toFilm, fromFilm } from '../placing/film-camera';
import { filmPlane } from '../placing/film-plane';
import { shapeBone } from '../placing/shape-bone';
import { placingOf } from '../placing/place-vertebra';
import { length, sub } from '../placing/vectors';
import { modelSpine } from './spine-model';
import { formationDrawing } from './formation-drawing';
import { axisEnds, formationMesh, growthRanks, threadFoot } from './formation-mesh';

it('grows along mesh paths from both seeds and keeps disconnected tips finite', () => {
  const ranks = growthRanks([[0, 0, 0], [2, 0, 0], [1, 0, 0], [1, 1, 0], [9, 9, 9]], [[0, 2], [1, 2], [2, 3]], [0, 1]);
  expect(ranks[0]).toBe(0); expect(ranks[1]).toBe(0); expect(ranks[2]).toBeLessThan(ranks[3] ?? 0); expect(ranks[4]).toBe(1);
  const surface = new Float32Array([-2, -2, -2, 2, -2, -2, 0, 2, -2, -2, -2, 2, 2, -2, 2, 0, 2, 2]);
  const mesh = formationMesh(surface, new Uint16Array([0, 1, 2, 3, 5, 4]), 4);
  expect(mesh.poles).toEqual([[0, 0, 2], [0, 0, -2]]);
  expect([...mesh.cell, ...mesh.turn, ...mesh.rank].every((v) => Number.isFinite(v) && v >= 0 && v <= 1)).toBe(true);
});

it('puts all seed points on each fitted surface and exactly at the ends of the film beams', () => {
  const bones = bonesFromDisk(), point = new Vector3(), closest = new Vector3(), triangle = new Triangle();
  let surfaceError = 0, projectionError = 0, crossingError = 0, packedError = 0, count = 0;
  for (const source of [MADE_UP_CASE, FRONT_ONLY_CASE]) {
    const front = fittedFilmCamera(source.front.geometry, source.front.size, source.fit.frontFilm);
    const side = source.side === null || source.fit.sideFilm === null ? null : fittedFilmCamera(source.side.geometry, source.side.size, source.fit.sideFilm);
    if (front === null) throw new Error('no-front-camera');
    for (const fitted of source.fit.vertebrae.filter((v) => v.level !== 'C7')) {
      const bone = bones.find((b) => b.level === fitted.level); if (bone === undefined) throw new Error('no-bone');
      const shaped = shapeBone(bone, fitted.pose), placing = placingOf(fitted.pose);
      for (const pole of axisEnds(shaped, bone.triangles, fitted.pose.height)) {
        count++; point.fromArray(pole); let nearest = Infinity;
        for (let i = 0; i < bone.triangles.length; i += 3) {
          triangle.a.fromArray(shaped, (bone.triangles[i] ?? 0) * 3); triangle.b.fromArray(shaped, (bone.triangles[i + 1] ?? 0) * 3); triangle.c.fromArray(shaped, (bone.triangles[i + 2] ?? 0) * 3);
          nearest = Math.min(nearest, triangle.closestPointToPoint(point, closest).distanceTo(point));
        }
        surfaceError = Math.max(surfaceError, nearest);
        const patient = placing.toPatient(pole);
        for (const camera of [front, side].filter((c) => c !== null)) {
          const plane = filmPlane(camera, -120), foot = threadFoot(patient, plane);
          const a = toFilm(camera, foot), b = toFilm(camera, patient);
          projectionError = Math.max(projectionError, Math.hypot(a[0] - b[0], a[1] - b[1]));
          crossingError = Math.max(crossingError, length(sub(fromFilm(camera, a, patient[0] * plane.towardReader[0] + patient[1] * plane.towardReader[1]), patient)));
          packedError = Math.max(packedError, Math.hypot(...patient.map((value) => Math.fround(value) - value)));
        }
      }
    }
  }
  console.info({ count, surfaceError, projectionError, crossingError, packedError });
  expect(count).toBe(68); expect(surfaceError).toBeLessThan(1e-10); expect(projectionError).toBeLessThan(1e-10); expect(crossingError).toBeLessThan(1e-10); expect(packedError).toBeLessThan(0.0001);
});

it('keeps one fixed point batch and two line batches, with identical packed seed and thread endpoints', () => {
  const model = modelSpine(FRONT_ONLY_CASE.fit, bonesFromDisk()), root = new Group();
  const drawing = formationDrawing(model, root, () => undefined);
  const front = fittedFilmCamera(FRONT_ONLY_CASE.front.geometry, FRONT_ONLY_CASE.front.size, FRONT_ONLY_CASE.fit.frontFilm);
  if (front === null) throw new Error('no-front-camera');
  for (const bone of model.bones) drawing.script(bone.level, { begins: 0, fills: 1000, first: 10, spread: [20, 100], loose: 0, lines: [100, 100], fade: [10, 100], reach: 10, hold: 20, threadFade: 100, pointFade: 10, pointSettle: 20, nodeSettle: 10 });
  drawing.threads([filmPlane(front, -120)]);
  const group = root.children[0]; if (group === undefined) throw new Error('no-drawing');
  const points = group.children.find((child) => child instanceof Points);
  const threads = group.children[2];
  if (!(points instanceof Points) || !(threads instanceof Mesh)) throw new Error('missing-batches');
  const pointGeometry = (points as Points).geometry, threadGeometry = (threads as Mesh).geometry;
  let dot = 0, thread = 0;
  for (const bone of model.bones) {
    for (let seed = 0; seed < 2; seed++) {
      const a = pointGeometry.getAttribute('position'), b = threadGeometry.getAttribute('aEnd');
      expect([a.getX(dot + seed), a.getY(dot + seed), a.getZ(dot + seed)]).toEqual([b.getX(thread * 4), b.getY(thread * 4), b.getZ(thread * 4)]); thread++;
    }
    dot += bone.formation.points.length;
  }
  const objects = [...group.children], geometries = objects.map((object) => (object as Mesh).geometry);
  for (const t of [0, 900, 300, 4020]) drawing.time(t, t < 4020);
  expect(group.visible).toBe(false); expect(group.children).toEqual(objects); expect(geometries.every((geometry, i) => (objects[i] as Mesh).geometry === geometry)).toBe(true); expect(group.children).toHaveLength(3);
  drawing.dispose(); model.dispose();
});
