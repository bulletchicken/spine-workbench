import { expect, it, vi } from 'vitest';
import { BackSide, BufferGeometry, CatmullRomCurve3, CylinderGeometry, DoubleSide, Float32BufferAttribute, Group, Line, LineLoop, LineSegments, Mesh, TubeGeometry, Vector3, type Material, type Object3D } from 'three';
import type { OnCanvas } from '../looking/lens';
import type { Vec3 } from '../placing/vectors';
import type { PartRegistration } from './scene-layer';
import { sceneLayer } from './scene-layer';
import { PART_RADIUS_PX, pickParts, visibleObject } from './part-picking';
import { columnCells, columnView } from './column-layout';
import { eyeDistance, toCanvas } from '../looking/lens';
import { namedView } from '../looking/framing';
import { ACCENT } from './look';

it('points at the actual surface within 12 CSS pixels, then depth, then registration, with nested and hidden objects', () => {
  const root = new Group(), inside = sceneLayer(root, new Map(), () => undefined, () => undefined), layer = inside.layer;
  const geometry = new BufferGeometry(); geometry.setAttribute('position', new Float32BufferAttribute([0, 0, 0, 0, 4, 0, 100, 0, 0], 3));
  const material = layer.lit(ACCENT), first = new Mesh(geometry, material), second = new Mesh(geometry, material), parent = new Group();
  parent.add(first); layer.onSpine.add(parent, second); layer.pointable(parent, 'first'); layer.pointable(second, 'second'); root.updateMatrixWorld(true);
  const project = (point: readonly [number, number, number]) => ({ x: point[0], y: point[1], nearer: point[2] });
  const hit = (x: number, y: number) => pickParts(inside.registrations(), project, x, y)?.registration.id;
  expect(hit(90, -12)).toBe('first'); expect(hit(90, -12.001)).toBeUndefined();
  second.position.z = 2; root.updateMatrixWorld(true); expect(hit(90, -5)).toBe('second');
  layer.pointable(first, 'child'); expect(hit(90, -5)).toBe('second');
  second.visible = false; expect(hit(90, -5)).toBe('child');
  parent.visible = false; expect(hit(90, -5)).toBeUndefined(); parent.visible = true;
  material.opacity = 0; expect(hit(90, -5)).toBeUndefined(); material.opacity = .001; expect(hit(90, -5)).toBe('child');
  first.removeFromParent(); expect(hit(90, -5)).toBeUndefined(); parent.add(first);
  const slanted = new BufferGeometry(); slanted.setAttribute('position', new Float32BufferAttribute([0, 0, 0, 0, 4, 8, 100, 0, 0], 3));
  first.geometry = slanted; second.visible = true; second.position.z = 4; root.updateMatrixWorld(true);
  expect(pickParts(inside.registrations(), project, 100 / 3, 4 / 3, 10)?.registration.id).toBe('child');
  first.geometry = geometry; second.visible = false; slanted.dispose();
  geometry.dispose(); expect(hit(90, -5)).toBeUndefined();
  layer.remove(); expect(inside.registrations()).toEqual([]);
});

it('lays out exact CSS cells and fits turned boxes at the smallest supported cell and after resize', () => {
  const box = { low: [-30, 0, -25] as const, high: [40, 400, 25] as const };
  for (const size of [{ width: 640, height: 700 }, { width: 1280, height: 720 }, { width: 576, height: 320 }]) for (const count of [1, 2, 6]) {
    const cells = columnCells(size, count, 0, 0), view = { ...namedView(box, size, 'front'), azimuth: 30, elevation: 20 };
    expect(cells.length).toBe(count);
    for (const cell of cells) {
      const camera = columnView(box, cell, size, view, box), centre = toCanvas(camera, size, [5, 200, 0]);
      expect(centre.x).toBeCloseTo((cell.left + cell.right) / 2); expect(centre.y).toBeCloseTo(size.height / 2);
      for (const point of [new Vector3(...box.low), new Vector3(...box.high)]) {
        const projected = toCanvas(camera, size, [point.x, point.y, point.z]);
        expect(projected.x).toBeGreaterThanOrEqual(cell.left - 1e-6); expect(projected.x).toBeLessThanOrEqual(cell.right + 1e-6);
        expect(projected.y).toBeGreaterThanOrEqual(cell.top - 1e-6); expect(projected.y).toBeLessThanOrEqual(cell.bottom + 1e-6);
      }
    }
  }
  expect(columnCells({ width: 208, height: 348 }, 2, 16, 28)).toEqual([{ left: 0, right: 96, top: 0, bottom: 320 }, { left: 112, right: 208, top: 0, bottom: 320 }]);
  expect(columnCells({ width: 640, height: 700 }, 0, 0, 0)).toEqual([]);
  expect(() => columnCells({ width: 640, height: 700 }, 2, -1, 0)).toThrow(RangeError);
});

/** Picking as it was first written: every triangle of every part, each corner projected afresh. The answer must not change. */
function pickedBefore(registrations: readonly PartRegistration[], project: (point: Vec3) => OnCanvas, x: number, y: number, eye: number | null): { registration: PartRegistration; distance: number; nearer: number } | null {
  const registered = new Set(registrations.map(({ object }) => object));
  let best: { registration: PartRegistration; distance: number; nearer: number } | null = null;
  const consider = (registration: PartRegistration, distance: number, nearer: number) => {
    if (!Number.isFinite(distance) || distance > PART_RADIUS_PX) return;
    if (best === null || distance < best.distance - 1e-7 || (Math.abs(distance - best.distance) <= 1e-7 && (nearer > best.nearer + 1e-7 || (Math.abs(nearer - best.nearer) <= 1e-7 && registration.order < best.registration.order)))) best = { registration, distance, nearer };
  };
  const depthAt = (points: readonly OnCanvas[], weights: readonly number[]) => eye === null ? points.reduce((sum, point, i) => sum + point.nearer * (weights[i] ?? 0), 0) : eye - 1 / points.reduce((sum, point, i) => sum + (weights[i] ?? 0) / (eye - point.nearer), 0);
  const segment = (a: OnCanvas, b: OnCanvas) => {
    const dx = b.x - a.x, dy = b.y - a.y, square = dx * dx + dy * dy;
    const t = square === 0 ? 0 : Math.max(0, Math.min(1, ((x - a.x) * dx + (y - a.y) * dy) / square));
    return { distance: Math.hypot(x - a.x - t * dx, y - a.y - t * dy), nearer: depthAt([a, b], [1 - t, t]) };
  };
  for (const registration of registrations) {
    const walk = (object: Object3D) => {
      if (!visibleObject(object) || (object !== registration.object && registered.has(object))) return;
      if (object instanceof Mesh || object instanceof Line) {
        const geometry = object.geometry as BufferGeometry, paints = object.material as Material | Material[];
        const position = geometry.getAttribute('position'), index = geometry.index;
        const vertex = (i: number) => { const v = new Vector3().fromBufferAttribute(position, index?.getX(i) ?? i).applyMatrix4(object.matrixWorld); return project([v.x, v.y, v.z]); };
        const start = geometry.drawRange.start, end = Math.min(index?.count ?? position.count, start + geometry.drawRange.count);
        const groups = Array.isArray(paints) ? geometry.groups : [{ start, count: end - start, materialIndex: 0 }];
        for (const group of groups) {
          const material = Array.isArray(paints) ? paints[group.materialIndex ?? 0] : paints;
          if (material === undefined || !material.visible || material.opacity <= 0) continue;
          const first = Math.max(start, group.start), last = Math.min(end, group.start + group.count);
          if (object instanceof Mesh) {
            for (let i = first; i + 2 < last; i += 3) {
              const a = vertex(i), b = vertex(i + 1), c = vertex(i + 2), area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
              if (Math.abs(area) < 1e-10) continue;
              if (material.side !== DoubleSide && (material.side === BackSide ? area < 0 : area > 0)) continue;
              const u = ((b.x - x) * (c.y - y) - (b.y - y) * (c.x - x)) / area, v = ((c.x - x) * (a.y - y) - (c.y - y) * (a.x - x)) / area, w = 1 - u - v;
              if (u >= 0 && v >= 0 && w >= 0) consider(registration, 0, depthAt([a, b, c], [u, v, w]));
              else for (const [from, to] of [[a, b], [b, c], [c, a]] as const) { const hit = segment(from, to); consider(registration, hit.distance, hit.nearer); }
            }
          } else {
            const step = object instanceof LineSegments ? 2 : 1;
            for (let i = first; i + 1 < last; i += step) { const hit = segment(vertex(i), vertex(i + 1)); consider(registration, hit.distance, hit.nearer); }
            if (object instanceof LineLoop && last > first) { const hit = segment(vertex(last - 1), vertex(first)); consider(registration, hit.distance, hit.nearer); }
          }
        }
      }
      for (const child of object.children) walk(child);
    };
    walk(registration.object);
  }
  return best;
}

it('picks the same part, at the same distance and depth, as picking every triangle did, and projects no point of a part the pointer is far from', () => {
  const root = new Group(), inside = sceneLayer(root, new Map(), () => undefined, () => undefined), layer = inside.layer;
  const screw = new CylinderGeometry(3, 3, 30, 12, 2), rod = new TubeGeometry(new CatmullRomCurve3([new Vector3(-20, 0, 0), new Vector3(-18, 60, 8), new Vector3(-21, 120, 4), new Vector3(-19, 180, -6)]), 40, 2.75, 10);
  const ring = new BufferGeometry(); ring.setAttribute('position', new Float32BufferAttribute(Array.from({ length: 24 }, (_, i) => [30 + 8 * Math.cos(i * Math.PI / 12), 90 + 8 * Math.sin(i * Math.PI / 12), 0]).flat(), 3));
  const seen = layer.lit(ACCENT), back = layer.lit(ACCENT); back.side = BackSide;
  for (let i = 0; i < 6; i++) {
    const part = new Mesh(screw, i % 3 === 2 ? back : seen); part.position.set(i % 2 === 0 ? -20 : 20, 20 + 28 * i, 5); part.rotation.set(Math.PI / 2, 0, (i - 3) * .2);
    layer.onSpine.add(part); layer.pointable(part, `anchor:${String(i)}`);
  }
  const tube = new Mesh(rod, seen), loop = new LineLoop(ring, layer.flat(ACCENT, 1));
  layer.onSpine.add(tube, loop); layer.pointable(tube, 'rod:left'); layer.pointable(loop, 'ring'); root.updateMatrixWorld(true);
  const box = { low: [-40, 0, -20] as const, high: [40, 200, 20] as const }, size = { width: 640, height: 720 };
  for (const view of [namedView(box, size, 'front'), { ...namedView(box, size, 'three-quarter'), azimuth: 146, elevation: 14, perspective: 1 }]) {
    const project = (point: Vec3) => toCanvas(view, size, point), eye = view.perspective < 1e-4 ? null : eyeDistance(view, size);
    const ids = new Set<string>();
    for (let x = 0; x <= size.width; x += 7) for (let y = 0; y <= size.height; y += 13) {
      const now = pickParts(inside.registrations(), project, x, y, eye), before = pickedBefore(inside.registrations(), project, x, y, eye);
      if (now !== null) ids.add(now.registration.id);
      expect(now === null ? null : { id: now.registration.id, distance: now.distance, nearer: now.nearer }).toEqual(before === null ? null : { id: before.registration.id, distance: before.distance, nearer: before.nearer });
    }
    // Every part is found somewhere, the screws drawn from behind included.
    expect(ids.size).toBe(8);
    // Far from every part, only the corners of each part's box are projected.
    const counted = vi.fn(project);
    expect(pickParts(inside.registrations(), counted, 2, 2, eye)).toBeNull();
    expect(counted).toHaveBeenCalledTimes(8 * inside.registrations().length);
  }
  layer.remove(); screw.dispose(); rod.dispose(); ring.dispose();
});
