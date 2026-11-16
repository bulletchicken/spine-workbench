import { afterEach, expect, it, vi } from 'vitest';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { MADE_UP_CASE } from '../made-up-case/made-up-case';
import type { CanvasSize } from '../looking/view';
import type { Vec3 } from '../placing/vectors';
import { makeStage } from './stage';
import { modelSpine } from './spine-model';

const observed = vi.hoisted(() => ({ resized: (): void => undefined }));
vi.mock('@/choreography/one-loop', () => ({ joinLoop: () => () => undefined }));
vi.mock('three', async () => {
  const actual = await vi.importActual<typeof import('three')>('three');
  class Renderer {
    autoClear = true; ratio = 1; width = 1; height = 1;
    setClearColor() { /* No raster surface in this test. */ }
    setPixelRatio(ratio: number) { this.ratio = ratio; }
    getPixelRatio() { return this.ratio; }
    setSize(width: number, height: number) { this.width = width; this.height = height; }
    getDrawingBufferSize(out: InstanceType<typeof actual.Vector2>) { return out.set(this.width * this.ratio, this.height * this.ratio); }
    setScissor() { /* As above. */ }
    setScissorTest() { /* As above. */ }
    setRenderTarget() { /* As above. */ }
    clear() { /* As above. */ }
    clearDepth() { /* As above. */ }
    render() { /* As above. */ }
    dispose() { /* No allocated graphics resources. */ }
  }
  return { ...actual, WebGLRenderer: Renderer };
});

afterEach(() => { vi.unstubAllGlobals(); });

function fixture() {
  vi.stubGlobal('window', { devicePixelRatio: 1 });
  vi.stubGlobal('ResizeObserver', class { constructor(callback: () => void) { observed.resized = callback; } observe() { /* Sizes are set below. */ } disconnect() { /* Nothing is watched. */ } });
  const canvas = { clientWidth: 640, clientHeight: 480 } as HTMLCanvasElement;
  const stage = makeStage(canvas), model = modelSpine(MADE_UP_CASE.fit, bonesFromDisk()), spine = stage.addSpine(model);
  const base = model.bones.map((bone) => bone.vertebra.pose);
  // A travelled spine is the one whose box is dear to work out.
  spine.betweenPoses(base, base.map((pose) => ({ ...pose, centre: [pose.centre[0] + 3, pose.centre[1] - 4, pose.centre[2]] })), 1);
  /** How many points of the bones' shapes are read while `act` runs. */
  const pointsRead = (act: () => void) => {
    const spies = model.bones.map((bone) => vi.spyOn(bone.geometry.getAttribute('position'), 'getX'));
    act();
    const count = spies.reduce((sum, spy) => sum + spy.mock.calls.length, 0);
    for (const spy of spies) spy.mockRestore();
    return count;
  };
  return { canvas, stage, model, spine, pointsRead };
}

const somePoints: Vec3[] = Array.from({ length: 200 }, (_, i) => [i % 7 - 3, i, (i % 11) - 5]);

it('works the box and the view out once however many points are projected, and again after each change', () => {
  const { canvas, stage, model, spine, pointsRead } = fixture();
  const rule = vi.fn((size: CanvasSize) => ({ ...stage.namedView('front'), aimAt: [size.width / 2, size.height / 2] as const }));
  stage.setView(rule);
  const box = stage.bounds();
  let runs = rule.mock.calls.length, before = somePoints.map((point) => stage.toCanvas(point));
  expect(rule.mock.calls.length - runs).toBeLessThanOrEqual(1);
  runs = rule.mock.calls.length;
  // Labels, rings, reach, walls and shadows all ask: nothing is worked out again.
  expect(pointsRead(() => {
    for (const point of somePoints) { stage.toCanvas(point); stage.patientToCanvas(point, spine); stage.patientToCanvas(point); stage.view(); stage.bounds(); }
  })).toBe(0);
  expect(rule.mock.calls.length).toBe(runs);
  expect(stage.bounds()).toBe(box);
  // A drawn frame asks the rule once more, and still walks no bone.
  expect(pointsRead(() => { stage.drawNow(); for (const point of somePoints) stage.toCanvas(point); })).toBe(0);
  expect(rule.mock.calls.length).toBe(runs + 1);
  expect(stage.bounds()).toBe(box);
  expect(somePoints.map((point) => stage.toCanvas(point))).toEqual(before);

  // A turn of the spine is a change: a new box, the rule asked again, and points land somewhere new.
  spine.setSpine({ turned: [25, 10] });
  runs = rule.mock.calls.length;
  const after = somePoints.map((point) => stage.toCanvas(point));
  expect(rule.mock.calls.length).toBe(runs + 1);
  expect(stage.bounds()).not.toBe(box);
  expect(after).not.toEqual(before);
  before = after;

  // So is a new size: the rule is given it.
  Object.defineProperty(canvas, 'clientWidth', { value: 320 }); observed.resized();
  expect(rule.mock.calls.at(-1)?.[0]).toEqual({ width: 320, height: 480 });
  expect(somePoints.map((point) => stage.toCanvas(point))).not.toEqual(before);

  // And so is a new view.
  const plain = { ...stage.view(), azimuth: 40 };
  stage.setView(plain);
  expect(stage.view()).toBe(plain);
  spine.remove(); stage.dispose(); model.dispose();
});

it('gives each spine in a column its own view, worked out once', () => {
  const { stage, model, spine } = fixture(), other = stage.addSpine(model, { shapesFrom: spine });
  stage.setView(() => stage.namedView('three-quarter'));
  stage.inColumns([spine, other], { gapPx: 16, footPx: 28 });
  const first = stage.patientToCanvas([0, 0, 0], spine), second = stage.patientToCanvas([0, 0, 0], other);
  expect(first.x).toBeLessThan(320); expect(second.x).toBeGreaterThan(320);
  const asked = vi.spyOn(spine, 'bounds');
  for (const point of somePoints) { stage.patientToCanvas(point, spine); stage.patientToCanvas(point, other); }
  expect(asked).not.toHaveBeenCalled();
  expect(stage.patientToCanvas([0, 0, 0], spine)).toEqual(first);
  stage.inColumns([], { gapPx: 0, footPx: 0 });
  expect(stage.patientToCanvas([0, 0, 0], spine)).not.toEqual(first);
  other.remove(); spine.remove(); stage.dispose(); model.dispose();
});
