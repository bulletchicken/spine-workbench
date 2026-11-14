import { afterEach, expect, it, vi } from 'vitest';
import { CylinderGeometry, Mesh, type Scene } from 'three';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { MADE_UP_CASE } from '../made-up-case/made-up-case';
import { ACCENT } from './look';
import { makeStage } from './stage';

const observed = vi.hoisted(() => ({ scene: null as Scene | null, passes: [] as number[], resized: (): void => undefined }));
vi.mock('@/choreography/one-loop', () => ({ joinLoop: () => () => undefined }));
vi.mock('three', async () => {
  const actual = await vi.importActual<typeof import('three')>('three');
  class Renderer {
    autoClear = true;
    ratio = 1;
    width = 1; height = 1;
    setClearColor() { /* No raster surface in this test. */ }
    setPixelRatio(ratio: number) { this.ratio = ratio; }
    getPixelRatio() { return this.ratio; }
    setSize(width: number, height: number) { this.width = width; this.height = height; }
    getDrawingBufferSize(out: InstanceType<typeof actual.Vector2>) { return out.set(this.width * this.ratio, this.height * this.ratio); }
    setScissor() { /* Cells are checked through projection and picking. */ }
    setScissorTest() { /* No raster surface in this test. */ }
    setRenderTarget() { /* Pass contents are inspected below. */ }
    clear() { /* No raster surface in this test. */ }
    clearDepth() { /* No raster surface in this test. */ }
    render(scene: Scene) {
      observed.scene = scene; let count = 0;
      scene.traverseVisible((object) => { if (object instanceof actual.Mesh) count++; }); observed.passes.push(count);
    }
    dispose() { /* No allocated graphics resources. */ }
  }
  return { ...actual, WebGLRenderer: Renderer };
});
import { modelSpine } from './spine-model';

afterEach(() => { vi.unstubAllGlobals(); observed.passes.length = 0; observed.scene = null; });

it('projects and picks per-column parts at 96 by 320 CSS pixels, shares shapes, resizes and restores ordinary layout', () => {
  vi.stubGlobal('window', { devicePixelRatio: 2 });
  vi.stubGlobal('ResizeObserver', class { constructor(callback: () => void) { observed.resized = callback; } observe() { /* Canvas dimensions are supplied below. */ } disconnect() { /* No observer remains. */ } });
  const canvas = { clientWidth: 208, clientHeight: 348 } as HTMLCanvasElement;
  const stage = makeStage(canvas), model = modelSpine(MADE_UP_CASE.fit, bonesFromDisk());
  const a = stage.addSpine(model), b = stage.addSpine(model, { shapesFrom: a });
  const ghost = stage.addSpine(model, { shapesFrom: a, flat: true }); ghost.setSpine({ strength: .07 });
  const levels = model.bones, bone = levels[4]; if (bone === undefined) throw new Error('Missing bone');
  const geometry = new CylinderGeometry(3, 3, 28, 8);
  const layers = [a, b].map((spine) => { const layer = spine.addLayer(); const object = new Mesh(geometry, layer.lit(ACCENT)); layer.onBone(bone.level).add(object); layer.pointable(object, 'anchor'); return layer; });
  stage.setView(() => stage.namedView('front'));
  const ordinary = stage.patientToCanvas(bone.placing.centre, a);
  stage.inColumns([a,b],{gapPx:16,footPx:28,progress:0});
  expect(stage.columnOf(a)).toEqual({left:0,right:208,top:0,bottom:348});
  stage.inColumns([a, b], { gapPx: 16, footPx: 28 });
  expect(stage.columnOf(a)).toEqual({ left: 0, right: 96, top: 0, bottom: 320 }); expect(stage.columnOf(ghost)).toBeNull();
  stage.drawNow(); expect(observed.passes.length).toBe(7);
  for (const [i, spine] of [a, b].entries()) {
    const point = stage.patientToCanvas(bone.placing.centre, spine), cell = stage.columnOf(spine);
    expect(point.x).toBeGreaterThan(cell?.left ?? Infinity); expect(point.x).toBeLessThan(cell?.right ?? -Infinity);
    expect(stage.pick(point.x, point.y)).toEqual({ kind: 'part', spine, layer: layers[i], id: 'anchor' });
  }
  const meshes: Mesh[] = []; observed.scene?.traverse((object) => { if (object instanceof Mesh && object.userData.level === bone.level) meshes.push(object as Mesh); });
  expect(meshes).toHaveLength(3); expect(new Set(meshes.map((mesh) => mesh.geometry)).size).toBe(1);
  const prior = stage.columnOf(a); expect(() => { stage.inColumns([a, a], { gapPx: 0, footPx: 0 }); }).toThrow(RangeError); expect(stage.columnOf(a)).toEqual(prior);
  Object.defineProperty(canvas, 'clientWidth', { value: 640 }); observed.resized(); expect(stage.columnOf(a)?.right).toBe(312);
  stage.inColumns([], { gapPx: 0, footPx: 0 }); expect(stage.columnOf(a)).toBeNull();
  Object.defineProperty(canvas, 'clientWidth', { value: 208 }); observed.resized(); expect(stage.patientToCanvas(bone.placing.centre, a)).toEqual(ordinary);
  layers.forEach((layer) => { layer.remove(); }); a.remove(); model.dispose();
  expect(stage.columnOf(a)).toBeNull(); expect(stage.pick(ordinary.x, ordinary.y)?.kind).not.toBe('part');
  b.remove(); ghost.remove(); stage.dispose(); geometry.dispose();
});
