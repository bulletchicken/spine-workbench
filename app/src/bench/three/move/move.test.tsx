import { afterEach, expect, it, vi } from 'vitest';
import type { Stage } from '@/spine-3d/scene/stage';
import type { DrawnSpine } from '@/spine-3d/scene/drawn-spine';
import type { MoveFilm } from '../../Stage';
import type { FilmWall } from '@/spine-3d/scene/film-wall';
import { filmCamera } from '@/spine-3d/placing/film-camera';
import { filmPlane } from '@/spine-3d/placing/film-plane';
import { viewAlongFilm } from '@/spine-3d/looking/framing';
import { toCanvas } from '@/spine-3d/looking/lens';
import type { View } from '@/spine-3d/looking/view';
import { toScene } from '@/spine-3d/placing/scene-axes';
import type { Vec3 } from '@/spine-3d/placing/vectors';
import { createMove } from './index';
const steps = vi.hoisted(() => new Set<(dt: number) => void>());
vi.mock('@/choreography/one-loop', () => ({ joinLoop: (step: (dt: number) => void) => { steps.add(step); return () => { steps.delete(step); }; } }));
vi.mock('@/choreography/timing', () => ({ durationOf: () => 160 }));
afterEach(() => { vi.unstubAllGlobals(); steps.clear(); document.body.replaceChildren(); });
it('has one seekable loop, settles fully on a press and consumes the resulting tile click', () => {
  vi.stubGlobal('matchMedia', () => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined }));
  const host = document.createElement('div'), stageElement = document.createElement('main'), canvas = document.createElement('canvas');
  host.append(canvas); document.body.append(host, stageElement);
  const clicked = vi.fn(); const tile = document.createElement('button'); tile.onclick = clicked; stageElement.append(tile);
  const view = { azimuth: 34, elevation: 14, perspective: 1, pxPerMm: [1, 1] as const, aim: [0, 0, 0] as const, aimAt: [300, 300] as const };
  const stage = { view: () => view, namedView: () => view, bounds: () => ({ low: [0, 0, 0], high: [100, 500, 100] }), setView: vi.fn(), film: () => null, drawNow: vi.fn() } as unknown as Stage;
  const set = vi.fn(), setSpine = vi.fn();
  const spine = { formation: { script: vi.fn(), threads: vi.fn(), press: vi.fn(), time: vi.fn() }, set, setSpine, levels: ['T1', 'L5'], about: () => ({ pose: { centre: [0, 0, 0] } }) } as unknown as DrawnSpine;
  const finish = vi.fn(), done = vi.fn();
  const move = createMove({ stage, spine, canvas, host, stageElement, films: { moment: 'three', compared: null, separation: 0, shown: () => null, opening: () => null, set: vi.fn(), finish }, done });
  move.play(); move.play(); expect(steps.size).toBe(1);
  for (const p of [.8, .2, 1, 0]) move.set(p);
  tile.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, cancelable: true })); tile.click();
  expect(clicked).not.toHaveBeenCalled(); expect(done).toHaveBeenCalledTimes(1); expect(finish).toHaveBeenCalledTimes(1);
  expect(set).toHaveBeenLastCalledWith({ cage: 0, pose: 1, reveal: 1, fill: 1, strength: 1, tint: 0 });
  expect(setSpine).toHaveBeenLastCalledWith(expect.objectContaining({ depth: 1, fill: 1, shadow: 1, scale: 1 }));
  expect(steps.size).toBe(0); expect(host.hasAttribute('data-moving')).toBe(false);
  move.dispose();
});

it('uses a plain fade for reduced motion and lets Enter settle without starting formation', () => {
  vi.stubGlobal('matchMedia', () => ({ matches: true, addEventListener: () => undefined, removeEventListener: () => undefined }));
  const host = document.createElement('div'), stageElement = document.createElement('main'), canvas = document.createElement('canvas'); host.append(canvas); document.body.append(host, stageElement);
  const view = { azimuth: 34, elevation: 14, perspective: 1, pxPerMm: [1, 1] as const, aim: [0, 0, 0] as const, aimAt: [300, 300] as const };
  const time = vi.fn(), done = vi.fn();
  const stage = { view: () => view, bounds: () => ({ low: [0, 0, 0], high: [100, 500, 100] }), namedView: () => view, setView: vi.fn(), film: () => null, drawNow: vi.fn() } as unknown as Stage;
  const spine = { formation: { script: vi.fn(), threads: vi.fn(), press: vi.fn(), time }, set: vi.fn(), setSpine: vi.fn(), levels: ['T1', 'L5'], about: () => ({ pose: { centre: [0, 0, 0] } }) } as unknown as DrawnSpine;
  const move = createMove({ stage, spine, canvas, host, stageElement, films: { moment: 'three', compared: null, separation: 0, shown: () => null, opening: () => null, set: vi.fn(), finish: vi.fn() }, done });
  move.play(); expect(host.style.opacity).toBe('0'); expect(time).not.toHaveBeenCalled();
  for (const step of steps) step(160); expect(host.style.opacity).toBe('0'); expect(time).not.toHaveBeenCalled();
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
  expect(done).toHaveBeenCalledOnce(); expect(time).toHaveBeenLastCalledWith(0, false); expect(host.style.opacity).toBe(''); expect(steps.size).toBe(0);
  move.dispose();
});


it('keeps the Front tile until the opening wall has had a frame to present', () => {
  vi.stubGlobal('matchMedia', () => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined }));
  const host = document.createElement('div'), stageElement = document.createElement('main'), canvas = document.createElement('canvas'), tile = document.createElement('div');
  tile.className = 'tile'; tile.dataset.place = 'front'; tile.style.setProperty('--bw', '280'); tile.style.setProperty('--bh', '560');
  stageElement.append(tile); host.append(canvas); document.body.append(host, stageElement);
  const camera = filmCamera({ mmPerPx: 1, mmPerPxVertical: null, quarterTurns: 0, imageRight: 'patient-left' }, [100, 200], [50, 100]);
  if (camera === null) throw new Error('no-camera');
  const shown = { pxPerFilmPx: 2.8, firstPixelAt: [501.4, 101.4] as const }, from = viewAlongFilm(camera, shown);
  const rest: View = { azimuth: 34, elevation: 14, perspective: 1, pxPerMm: [1, 1], aim: [0, 170, 0], aimAt: [640, 400] };
  const placement = { size: { width: 100, height: 200 }, view: { quarterTurns: 0 as const, mirrored: false }, zoom: 2.8, pan: [0, 0] as const };
  const source: MoveFilm = { film: { size: placement.size, pixels: new Uint16Array(20000), thumbnail: new Uint8Array(), outlines: [], names: true, on: null, only: null, frame: { kind: 'whole' }, light: null }, opening: { x: 500, y: 100, width: 280, height: 560, placement }, rail: { x: 50, y: 300, width: 66, height: 132, placement: { ...placement, zoom: .66 } } };
  let corners: readonly Vec3[] = [], view = from, picture: unknown = null, presented = 0;
  const wallSet = vi.fn(), press = vi.fn();
  const wall = { camera, plane: () => filmPlane(camera, -30), values: () => ({ strength: 0, out: null, corner: 8, hinge: 1 }), set: wallSet, show: (pixels: unknown) => { picture = pixels; }, place: (points: readonly Vec3[] | null) => { corners = points ?? []; }, marks: vi.fn() } as unknown as FilmWall;
  const size = { width: 1280, height: 800 };
  const drawNow = vi.fn(() => {
    expect(tile.style.opacity).not.toBe('0'); expect(tile.style.getPropertyValue('--bw')).toBe('280');
    expect(picture).toEqual({ data: source.film.pixels, width: 100, height: 200 });
    expect(corners).toHaveLength(4);
    corners.forEach((corner, i) => { const pixel = toCanvas(view, size, toScene(corner)); expect(pixel.x).toBeCloseTo(500 + (i === 1 || i === 2 ? 280 : 0), 10); expect(pixel.y).toBeCloseTo(100 + (i >= 2 ? 560 : 0), 10); });
  });
  const stage = { view: () => from, viewAlong: () => from, namedView: () => rest, bounds: () => ({ low: [-50, 0, -20], high: [50, 400, 20] }), size: () => size, setView: (next: View) => { view = next; }, toCanvas: (point: Vec3) => toCanvas(view, size, point), film: (name: string) => name === 'front' ? wall : null, drawNow } as unknown as Stage;
  const spine = { formation: { script: vi.fn(), threads: vi.fn(), press, time: vi.fn() }, levels: ['T1'], set: vi.fn(), setSpine: vi.fn(), pressOnto: vi.fn() } as unknown as DrawnSpine;
  const set = vi.fn(() => { expect(presented).toBeGreaterThan(0); tile.style.setProperty('--bw', '66'); });
  const move = createMove({ stage, spine, canvas, host, stageElement, films: { moment: 'three', compared: null, separation: 0, shown: () => shown, opening: () => shown, moveFilm: (name) => name === 'front' ? source : null, set, finish: vi.fn() }, done: vi.fn() });
  expect(tile.style.opacity).not.toBe('0'); move.play();
  expect(drawNow).toHaveBeenCalledOnce(); expect(set).not.toHaveBeenCalled(); expect(tile.style.opacity).not.toBe('0');
  const frame = [...steps][0]; if (frame === undefined) throw new Error('no-frame');
  frame(0);
  expect(drawNow).toHaveBeenCalledTimes(2); expect(set).not.toHaveBeenCalled();
  expect(tile.style.opacity).not.toBe('0'); expect(tile.style.getPropertyValue('--bw')).toBe('280');
  // The browser paints after this callback returns, before the next frame callback.
  presented++;
  frame(16);
  expect(drawNow).toHaveBeenCalledTimes(3); expect(set).toHaveBeenCalledOnce(); expect(tile.style.opacity).toBe('0');
  // Front alone: the bones form pressed onto the standing wall, over their outlines.
  const out = (wallSet.mock.calls.find(([values]) => typeof values === 'object' && values !== null && 'out' in values)?.[0] as { out: number } | undefined)?.out;
  expect(out).toBeDefined();
  expect(press).toHaveBeenCalledWith({ ...filmPlane(camera, out ?? 0) });
  move.dispose();
  expect(press).toHaveBeenLastCalledWith(null);
});
