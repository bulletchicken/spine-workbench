import { expect, it, vi } from 'vitest';
import { CylinderGeometry, Group, Mesh, Object3D, Vector3, MeshBasicMaterial } from 'three';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { MADE_UP_CASE } from '../made-up-case/made-up-case';
import { placingOf } from '../placing/place-vertebra';
import { poseBetween } from '../placing/pose-between';
import { moved, pressedOnto } from '../placing/flat-on-film';
import { ACCENT } from './look';
import { drawSpine } from './drawn-spine';
import { modelSpine } from './spine-model';

const poses = MADE_UP_CASE.fit.vertebrae.map((v) => v.pose);
function fixture() {
  const scene = new Group(), changed = vi.fn();
  const model = modelSpine(MADE_UP_CASE.fit, bonesFromDisk());
  const inside = drawSpine(model, { scene, changed, removed: vi.fn() });
  return { scene, model, inside, spine: inside.spine, changed };
}
function close(actual: readonly number[], expected: readonly number[]) { actual.forEach((n, i) => { expect(n).toBeCloseTo(expected[i] ?? 0, 7); }); }

it('samples straight centres and the shortest turn at 0, half and 1, including across pi', () => {
  const base = poses[0]; if (base === undefined) throw new Error('Missing pose');
  const from = { ...base, centre: [0, 0, 0] as [number, number, number], rotation: [0, 0, 0] as [number, number, number] };
  const to = { ...from, centre: [10, 20, 30] as [number, number, number], rotation: [0, 0, Math.PI / 2] as [number, number, number] };
  for (const p of [0, .5, 1]) {
    const pose = poseBetween(from, to, p);
    close(pose.centre, [10 * p, 20 * p, 30 * p]);
    close(placingOf(pose).toPatient([1, 0, 0]), [10 * p + Math.cos(p * Math.PI / 2), 20 * p + Math.sin(p * Math.PI / 2), 30 * p]);
  }
  const half = poseBetween({ ...from, rotation: [0, 0, Math.PI * 170 / 180] }, { ...from, rotation: [0, 0, -Math.PI * 170 / 180] }, .5);
  close(placingOf(half).toPatient([1, 0, 0]), [-1, 0, 0]);
});

it('keeps layer frames with the drawn bone through pose travel, root movement and pressing onto a film', () => {
  const { scene, model, inside, spine } = fixture(), layer = spine.addLayer();
  const level = spine.levels[0]; if (level === undefined) throw new Error('Missing level');
  const own = [3, 4, 5] as const, point = new Object3D(); point.position.set(...own); layer.onBone(level).add(point);
  expect(layer.onBone(level)).toBe(layer.onBone(level));
  const baseline = scene.children.length;
  expect(() => layer.onBone('C7')).toThrow(/C7/); expect(scene.children.length).toBe(baseline);
  spine.setSpine({ offset: [7, 8, 9], scale: 1.3, turned: [20, 10] });
  const to = poses.map((pose) => ({ ...pose, centre: [pose.centre[0] + 20, pose.centre[1] - 10, pose.centre[2] + 30] as [number, number, number], rotation: [0, 0, Math.PI / 2] as [number, number, number] }));
  for (const p of [0, .5, 1, .5]) {
    spine.betweenPoses(poses, to, p); scene.updateMatrixWorld(true);
    const fromPose = poses[0], toPose = to[0]; if (fromPose === undefined || toPose === undefined) throw new Error('Missing pose');
    const patient = placingOf(poseBetween(fromPose, toPose, p)).toPatient(own);
    close(point.getWorldPosition(new Vector3()).toArray(), spine.inScene(patient));
    expect(spine.bounds().low.every(Number.isFinite)).toBe(true);
  }
  const before = point.getWorldPosition(new Vector3()).toArray();
  expect(() => { spine.betweenPoses(poses.slice(1), to, .5); }).toThrow(RangeError);
  scene.updateMatrixWorld(true); close(point.getWorldPosition(new Vector3()).toArray(), before);
  const plane = { towardReader: [1, 0, 0] as const, out: -120 };
  spine.pressOnto(plane); spine.set({ pose: .5 }); spine.setSpine({ depth: .4 });
  const a = poses[0], b = to[0]; if (a === undefined || b === undefined) throw new Error('Missing pose');
  const expected = moved(pressedOnto(plane, .2), placingOf(poseBetween(a, b, .5)).toPatient(own));
  scene.updateMatrixWorld(true); close(point.getWorldPosition(new Vector3()).toArray(), spine.inScene(expected));
  layer.remove(); spine.remove(); model.dispose();
  expect(inside.layers).toHaveLength(0);
});

it('separates the bone veil from spine fading and removes only the resources owned by the layer', () => {
  const { model, inside, spine } = fixture();
  const count = inside.root.children.length, layer = spine.addLayer(), material = layer.flat(ACCENT, .8), shape = new CylinderGeometry(2, 2, 20);
  const object = new Mesh(shape, material); layer.onSpine.add(object); layer.pointable(object, 'rod');
  const stopWatching = vi.spyOn(shape, 'removeEventListener');
  const freeShape = vi.spyOn(shape, 'dispose'), freeMaterial = vi.spyOn(material, 'dispose');
  for (const strength of [.55, .34]) {
    spine.set({ strength }); inside.pass('rest'); expect(material.opacity).toBe(.8);
    inside.pass('opaque-depth'); expect(inside.pickable()).toHaveLength(0);
    inside.pass('rest'); expect(inside.pickable()).toHaveLength(model.bones.length);
    layer.onSpine.visible = false; inside.pass('base'); inside.pass('parts'); inside.pass('rest'); expect(layer.onSpine.visible).toBe(false); layer.onSpine.visible = true;
  }
  spine.setSpine({ strength: .5 }); expect(material.opacity).toBe(.4);
  spine.setSpine({ strength: .5 }); expect(material.opacity).toBe(.4);
  layer.inFront(object, true); inside.pass('rest'); expect(object.material).not.toBe(material); expect(object.material.depthTest).toBe(true);
  layer.inFront(object, false); inside.pass('rest'); expect(object.material).toBe(material);
  layer.remove(); layer.remove(); layer.changed();
  expect(inside.root.children.length).toBe(count); expect(inside.layers).toHaveLength(0); expect(freeMaterial).toHaveBeenCalledOnce(); expect(freeShape).not.toHaveBeenCalled(); expect(stopWatching).toHaveBeenCalledOnce();
  spine.remove(); spine.remove(); model.dispose(); shape.dispose();
});

it('shares every shaped bone across six drawings and keeps it alive after source removal until the last drawing leaves', () => {
  const { model, inside, spine, scene } = fixture(), free = model.bones.map((bone) => vi.spyOn(bone.geometry, 'dispose'));
  const drawings = [inside, ...Array.from({ length: 5 }, (_, i) => drawSpine(model, { scene, changed: vi.fn(), removed: vi.fn() }, i === 4))];
  for (const drawing of drawings.slice(1)) expect(drawing.root.children[0]?.children[1]).toBeInstanceOf(Mesh);
  for (const drawing of drawings) {
    const solid = drawing.root.children[0]?.children[1]; if (!(solid instanceof Mesh)) throw new Error('Missing surface');
    expect(solid.geometry).toBe(model.bones[0]?.geometry);
  }
  const ghost = drawings.at(-1); if (ghost === undefined) throw new Error('Missing ghost');
  const solid = ghost.root.children[0]?.children[1]; if (!(solid instanceof Mesh)) throw new Error('Missing surface');
  expect(solid.material).toBeInstanceOf(MeshBasicMaterial); expect(ghost.pickable()).toEqual([]);
  spine.remove(); model.dispose(); expect(free.every((spy) => spy.mock.calls.length === 0)).toBe(true);
  drawings.slice(1).forEach((drawing) => { drawing.spine.remove(); }); model.dispose();
  expect(free.every((spy) => spy.mock.calls.length === 1)).toBe(true);
});
