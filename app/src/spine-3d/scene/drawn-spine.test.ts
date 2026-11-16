import { expect, it, vi } from 'vitest';
import { Group } from 'three';
import type { VertebraPose } from '@/shapes';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { MADE_UP_CASE } from '../made-up-case/made-up-case';
import { placingOf } from '../placing/place-vertebra';
import { poseBetween } from '../placing/pose-between';
import type { Vec3 } from '../placing/vectors';
import { drawSpine } from './drawn-spine';
import { modelSpine, type SpineModel } from './spine-model';

const model = modelSpine(MADE_UP_CASE.fit, bonesFromDisk());
const base = model.bones.map((bone) => bone.vertebra.pose);
/** The fitted poses, each bone moved and turned a little, as a load moves them. */
const travelled: VertebraPose[] = base.map((pose, i) => ({ ...pose, centre: [pose.centre[0] + i, pose.centre[1] - 2 * i, pose.centre[2] + 1], rotation: [pose.rotation[0] + .01 * i, pose.rotation[1] - .02, pose.rotation[2] + .03] }));

function fixture(on: SpineModel = model) {
  const changed = vi.fn(), inside = drawSpine(on, { scene: new Group(), changed, removed: vi.fn() });
  return { inside, spine: inside.spine, changed };
}

/** The box every point of every bone makes, worked out point by point through the spine's own `inScene`, as it was before any box was kept. */
function pointByPoint(spine: ReturnType<typeof fixture>['spine'], poses: readonly VertebraPose[], p: number) {
  const low = [Infinity, Infinity, Infinity], high = [-Infinity, -Infinity, -Infinity];
  for (const bone of model.bones) {
    const from = base.find((pose) => pose.level === bone.level), to = poses.find((pose) => pose.level === bone.level);
    if (from === undefined || to === undefined) throw new Error('Missing pose');
    const placing = placingOf(poseBetween(from, to, p)), points = bone.geometry.getAttribute('position');
    for (let i = 0; i < points.count; i++) {
      const at = spine.inScene(placing.toPatient([points.getX(i), points.getY(i), points.getZ(i)]));
      for (const axis of [0, 1, 2] as const) { low[axis] = Math.min(low[axis] ?? Infinity, at[axis]); high[axis] = Math.max(high[axis] ?? -Infinity, at[axis]); }
    }
  }
  return { low, high };
}

/** How many points of the bones' shapes are read while `act` runs. */
function pointsRead(act: () => void) {
  const spies = model.bones.map((bone) => vi.spyOn(bone.geometry.getAttribute('position'), 'getX'));
  act();
  const count = spies.reduce((sum, spy) => sum + spy.mock.calls.length, 0);
  for (const spy of spies) spy.mockRestore();
  return count;
}

it('gives a travelled spine the same box as working it out point by point, after every turn, size and move', () => {
  const { spine } = fixture();
  spine.betweenPoses(base, travelled, .6);
  expect(spine.bounds()).toEqual(pointByPoint(spine, travelled, .6));
  spine.setSpine({ turned: [30, 12] });
  expect(spine.bounds()).toEqual(pointByPoint(spine, travelled, .6));
  spine.setSpine({ scale: .7 });
  expect(spine.bounds()).toEqual(pointByPoint(spine, travelled, .6));
  spine.setSpine({ offset: [12, -5, 40] as Vec3 });
  expect(spine.bounds()).toEqual(pointByPoint(spine, travelled, .6));
  spine.betweenPoses(base, travelled, 1);
  expect(spine.bounds()).toEqual(pointByPoint(spine, travelled, 1));
  spine.remove();
});

it('works the box out once for each change, and not again until the spine moves', () => {
  const { spine } = fixture();
  spine.betweenPoses(base, travelled, 1);
  let box = spine.bounds();
  expect(pointsRead(() => { for (let i = 0; i < 50; i++) expect(spine.bounds()).toBe(box); })).toBe(0);
  // Strength, tint and fill change nothing about where the spine is.
  spine.setSpine({ strength: .4, shadow: .5 }); spine.set({ tint: 1, strength: .3, fill: .5 });
  expect(pointsRead(() => { expect(spine.bounds()).toBe(box); })).toBe(0);
  for (const change of [() => { spine.setSpine({ turned: [10, 5] }); }, () => { spine.setSpine({ scale: 1.2 }); }, () => { spine.setSpine({ offset: [1, 2, 3] }); }, () => { spine.setSpine({ depth: .5 }); }, () => { spine.set({ reveal: .5 }); }, () => { spine.betweenPoses(base, travelled, .5); }]) {
    change();
    expect(pointsRead(() => { expect(spine.bounds()).not.toBe(box); box = spine.bounds(); expect(spine.bounds()).toBe(box); })).toBeGreaterThan(0);
  }
  // The same values set again are no change.
  spine.setSpine({ turned: [10, 5], scale: 1.2, offset: [1, 2, 3] });
  expect(spine.bounds()).toBe(box);
  spine.remove();
});

it('stops counting as travelled once every bone is back on the model, and is then boxed and drawn as one that never moved', () => {
  const { spine, inside } = fixture(), untouched = fixture();
  spine.setSpine({ turned: [20, 10] }); untouched.spine.setSpine({ turned: [20, 10] });
  spine.betweenPoses(base, travelled, 1);
  const version = inside.version();
  spine.betweenPoses(base, base, 1);
  expect(inside.version()).toBeGreaterThan(version);
  expect(pointsRead(() => { expect(spine.bounds()).toEqual(untouched.spine.bounds()); })).toBe(0);
  for (const level of spine.levels) expect(spine.centre(level)).toEqual(untouched.spine.centre(level));
  inside.root.updateMatrixWorld(true); untouched.inside.root.updateMatrixWorld(true);
  expect(inside.root.children.map((child) => child.matrixWorld.elements)).toEqual(untouched.inside.root.children.map((child) => child.matrixWorld.elements));
  // A move that starts from the model's poses and has not left them yet is not travel either; one that has is.
  spine.betweenPoses(base, travelled, 0);
  expect(pointsRead(() => { spine.bounds(); })).toBe(0);
  spine.betweenPoses(travelled, base, .5);
  expect(pointsRead(() => { spine.bounds(); })).toBeGreaterThan(0);
  spine.betweenPoses(travelled, base, 1);
  expect(pointsRead(() => { expect(spine.bounds()).toEqual(untouched.spine.bounds()); })).toBe(0);
  spine.remove(); untouched.spine.remove();
});
