import { expect, it } from 'vitest';
import { BufferGeometry, Float32BufferAttribute, Group, Mesh, Vector3 } from 'three';
import { sceneLayer } from './scene-layer';
import { pickParts } from './part-picking';
import { columnCells, columnView } from './column-layout';
import { toCanvas } from '../looking/lens';
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
