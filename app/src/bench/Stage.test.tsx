import { act, type ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { FilmKind } from '@/shapes';
import { cameras } from '@/film-viewer/camera/cameras';
import type { DrawState, FilmPainter } from '@/film-viewer/gpu/film-painter';
import { filmSpace } from '@/film-viewer/film-space';
import type { FilmSurface } from '@/film-viewer/gpu/gl-context';
import { Stage, type FilmDriver, type StageFilm, type StageProps } from './Stage';
import { FILM_ORDER } from './stage-layout';

const { steps, painters } = vi.hoisted(() => ({
  steps: new Set<(dt: number, now: number) => void>(),
  painters: new Set<(state: DrawState) => void>(),
}));
vi.mock('@/choreography/one-loop', () => ({ joinLoop: (step: (dt: number, now: number) => void) => { steps.add(step); return () => { steps.delete(step); }; } }));
vi.mock('@/choreography/timing', () => ({
  durationOf: (_part: Element, name: string) => name === '--dur-push' ? 800 : 160,
  curveOf: () => (t: number) => t,
  numberOf: (_part: Element, name: string) => ({ '--name-room': 24, '--film-h': 1300, '--light-in': 0.05, '--light-out': 0.93 })[name],
}));
vi.mock('@/film-viewer/gpu/film-painter', () => ({
  paintFilm: (surface: FilmSurface, _corner: number, told: (state: DrawState) => void): FilmPainter => {
    painters.add(told);
    return { setFilm: () => undefined, setLook: () => undefined, resize: (width, height) => { surface.width = width; surface.height = height; }, draw: () => undefined, state: 'drawn', blending: null, stop: () => { painters.delete(told); } };
  },
}));

globalThis.IS_REACT_ACT_ENVIRONMENT = true;

let width = 1280;
let height = 672;
let offset = 0;
let reduced = false;
class StageObserver {
  static all: StageObserver[] = [];
  element: Element | null = null;
  constructor(readonly notify: ResizeObserverCallback) { StageObserver.all.push(this); }
  observe(element: Element) { this.element = element; }
  disconnect() { this.element = null; }
}
function resized() {
  act(() => {
    for (const observer of StageObserver.all) {
      const element = observer.element;
      if (element === null) continue;
      const box = DOMRect.fromRect({ width: element.clientWidth, height: element.clientHeight });
      observer.notify([{ target: element, contentRect: box, borderBoxSize: [], contentBoxSize: [], devicePixelContentBoxSize: [] }], observer as unknown as ResizeObserver);
    }
  });
}
function tick(dt = 16) { act(() => { for (const step of [...steps]) if (steps.has(step)) step(dt, dt); }); }

let host: HTMLDivElement;
let root: Root | null;
let props: StageProps;
const empty: Record<FilmKind, StageFilm | null> = { front: null, side: null, 'bend-left': null, 'bend-right': null };
const madeUpFilm = (): StageFilm => ({
  size: { width: 8, height: 16 }, pixels: new Uint16Array(128), thumbnail: new Uint8Array([137, 80, 78, 71]),
  outlines: [], names: false, on: null, only: null, frame: { kind: 'whole' }, light: null,
});
function find(selector: string): HTMLElement {
  const found = host.querySelector<HTMLElement>(selector);
  if (found === null) throw new Error(`no ${selector}`);
  return found;
}
async function show(changes: Partial<StageProps> = {}) {
  props = { ...props, ...changes };
  await act(async () => {
    root?.render(<main className="stage" aria-label="The films"><Stage {...props} /></main>);
    await Promise.resolve();
  });
  resized();
}
function render(node: ReactNode) { act(() => { root?.render(node); }); }

beforeEach(() => {
  width = 1280; height = 672; offset = 0; reduced = false;
  StageObserver.all = []; steps.clear(); painters.clear();
  vi.stubGlobal('ResizeObserver', StageObserver);
  vi.stubGlobal('matchMedia', () => ({ matches: reduced, addEventListener: () => undefined, removeEventListener: () => undefined }));
  Object.defineProperty(document, 'fonts', { configurable: true, value: { ready: Promise.resolve() } });
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(function (this: HTMLElement) { return this.classList.contains('stage') ? width : Number(this.closest<HTMLElement>('.tile')?.style.getPropertyValue('--bw') ?? 0); });
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(function (this: HTMLElement) { return this.classList.contains('stage') ? height : Number(this.closest<HTMLElement>('.tile')?.style.getPropertyValue('--bh') ?? 0); });
  vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(() => width >= 916 ? 48 : 96);
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) { return DOMRect.fromRect({ x: offset, y: 88 + offset, width: this.clientWidth, height: this.clientHeight }); });
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  props = { moment: 'films', films: { ...empty }, reading: [], held: { over: false, target: null }, leaving: false, marked: false, reviewing: false, replay: null, onPressPlace: vi.fn() };
});
afterEach(() => {
  act(() => { root?.unmount(); }); root = null; host.remove();
  for (const kind of FILM_ORDER) cameras.drop(kind);
  vi.unstubAllGlobals(); vi.restoreAllMocks(); vi.useRealTimers();
});

describe('the stage', () => {
  it('has no extra root, four places in film order, and the display sentence', async () => {
    await show();
    expect([...find('main').children].map((part) => part.className)).toEqual(['display', 'dropzone', 'tiles']);
    expect(find('h1.display').textContent).toBe('Turn four X-rays into a spine you can measure.');
    expect([...host.querySelectorAll('.place')].map((part) => part.getAttribute('aria-label'))).toEqual(['Choose the Front film', 'Choose the Side film', 'Choose the Bending left film', 'Choose the Bending right film']);
    expect([...host.querySelectorAll('.place')].map((part) => part.getAttribute('data-place'))).toEqual(FILM_ORDER);
    expect(host.querySelectorAll('.place svg rect')).toHaveLength(28);
    act(() => { find('.place[data-place="side"]').click(); });
    expect(props.onPressPlace).toHaveBeenCalledWith('side');
    expect(host.querySelectorAll('.caption')).toHaveLength(4);
  });
  it('lands a film in its real box, with its names flag and a filled replacement place', async () => {
    await show({ films: { ...empty, front: madeUpFilm() } });
    const tile = find('.tile.in[data-place="front"]');
    expect(tile.style.getPropertyValue('--s')).toBe('1');
    expect(tile.style.getPropertyValue('--bw')).toBe('208');
    expect(tile.style.getPropertyValue('--bh')).toBe('416');
    expect(tile.getAttribute('data-names')).toBe('off');
    expect(find('.place.filled').getAttribute('aria-label')).toBe('Replace the Front film');
    expect(find('.caption').getAttribute('aria-hidden')).toBe('true');
    expect(find('canvas').getAttribute('aria-label')).toBe('Front film');
    expect(cameras.frameOf('front')?.view).toEqual({ quarterTurns: 0, mirrored: false });
    expect(cameras.zoomOverFitOf('front')).toBe(1);
    expect(find('.film > .hung-film')).not.toBeNull();
  });
  it('names reading places, disables presses during intake, and follows empty, landing and ready', async () => {
    await show();
    expect(document.documentElement.dataset.films).toBe('empty');
    await show({ reading: ['side'] });
    expect(document.documentElement.dataset.films).toBe('landing');
    expect(find('.place[data-reading]').getAttribute('aria-label')).toBe('Reading the Side film');
    expect(find('.place[data-reading]').getAttribute('aria-busy')).toBe('true');
    expect([...host.querySelectorAll<HTMLButtonElement>('.place')].every((part) => part.disabled)).toBe(true);
    await show({ reading: [], films: { front: madeUpFilm(), side: madeUpFilm(), 'bend-left': madeUpFilm(), 'bend-right': madeUpFilm() } });
    expect(document.documentElement.dataset.films).toBe('ready');
    expect([...host.querySelectorAll<HTMLButtonElement>('.place')].every((part) => !part.disabled)).toBe(true);
  });
  it('writes the page attributes after fonts and layout, and removes them on unmount', async () => {
    await show({ held: { over: true, target: null }, leaving: true });
    for (const name of ['ready', 'drag', 'leaving']) expect(document.documentElement.hasAttribute(`data-${name}`)).toBe(true);
    await show({ held: { over: false, target: null }, leaving: false });
    expect(document.documentElement.hasAttribute('data-drag')).toBe(false);
    expect(document.documentElement.hasAttribute('data-leaving')).toBe(false);
    render(null);
    for (const name of ['films', 'ready', 'drag', 'leaving']) expect(document.documentElement.hasAttribute(`data-${name}`)).toBe(false);
  });
  it('marks the held destination on its place and tile, and keeps state on the tiles', async () => {
    await show({ films: { ...empty, front: madeUpFilm() }, held: { over: true, target: 'front' }, marked: true, reviewing: true });
    expect(host.querySelectorAll('[data-drop-target]')).toHaveLength(2);
    expect(find('.tiles').classList.contains('marked')).toBe(true);
    expect(find('.tiles').classList.contains('reviewing')).toBe(true);
    await show({ held: { over: true, target: 'side' } });
    expect(find('[data-drop-target]').getAttribute('data-place')).toBe('side');
    expect(host.querySelectorAll('[data-drop-target]')).toHaveLength(1);
  });
  it('carries the same film and caption into Find, hides absent names and ends the flight', async () => {
    const front = madeUpFilm();
    await show({ films: { ...empty, front } });
    const tile = find('.tile'); const viewer = find('.film-viewer'); const caption = find('.caption');
    await show({ moment: 'find', films: { ...empty, front: { ...front, names: true } } });
    expect(find('.tile')).toBe(tile); expect(find('.film-viewer')).toBe(viewer); expect(find('.caption')).toBe(caption);
    expect(find('.tiles').hasAttribute('data-flying')).toBe(true);
    expect(find('.tiles').inert).toBe(true);
    expect(tile.style.getPropertyValue('--s')).toBe('1');
    expect(tile.style.getPropertyValue('--bw')).toBe('322');
    expect(tile.style.getPropertyValue('--x')).toBe('479');
    expect(tile.getAttribute('data-names')).toBe('on');
    expect(host.querySelectorAll('.caption[data-away]')).toHaveLength(3);
    expect([...host.querySelectorAll<HTMLButtonElement>('.place')].every((part) => part.disabled)).toBe(true);
    act(() => { tile.dispatchEvent(Object.assign(new Event('transitionend', { bubbles: true }), { propertyName: 'transform' })); });
    expect(find('.tiles').hasAttribute('data-flying')).toBe(false);
    expect(find('.tiles').inert).toBe(false);
    expect(caption.style.getPropertyValue('--cx')).toBe('640');
    expect(caption.style.getPropertyValue('--cy')).toBe('654');
  });
  it('uses a resize without flight and keeps film edges on whole window pixels', async () => {
    offset = 0.5;
    await show({ films: { ...empty, front: madeUpFilm() } });
    expect(find('.tile').style.getPropertyValue('--x')).toBe('187.5');
    expect(find('.tile').style.getPropertyValue('--y')).toBe('161.5');
    width = 608; height = 572; resized();
    expect(find('.tile').style.getPropertyValue('--bw')).toBe('134');
    expect(find('.tile').style.getPropertyValue('--x')).toBe('-0.5');
    expect(find('.tile').style.getPropertyValue('--y')).toBe('209.5');
    expect(find('.tiles').hasAttribute('data-flying')).toBe(false);
    expect(find('.tiles').classList.contains('still')).toBe(false);
  });
  it.each(['system', 'app'])('fades around a layout change with reduced motion from the %s', async (source) => {
    vi.useFakeTimers();
    reduced = source === 'system';
    if (source === 'app') host.setAttribute('data-motion', 'reduced');
    await show({ films: { ...empty, front: madeUpFilm() } });
    await show({ moment: 'find' });
    expect(find('.tiles').classList.contains('fading')).toBe(true);
    expect(find('.tile').style.getPropertyValue('--bw')).toBe('208');
    act(() => { vi.advanceTimersByTime(159); });
    expect(find('.tile').style.getPropertyValue('--bw')).toBe('208');
    act(() => { vi.advanceTimersByTime(1); });
    expect(find('.tile').style.getPropertyValue('--bw')).toBe('322');
    expect(find('.tiles').classList.contains('fading')).toBe(false);
    expect(find('.tiles').hasAttribute('data-flying')).toBe(false);
  });
  it('remeasures a capped frame’s half-pixel offset when only its position changes', async () => {
    offset = 0.5;
    await show({ films: { ...empty, front: madeUpFilm() } });
    expect(find('.tile').style.getPropertyValue('--x')).toBe('187.5');
    offset = 0;
    act(() => { window.dispatchEvent(new Event('resize')); });
    expect(find('.tile').style.getPropertyValue('--x')).toBe('188');
    expect(find('.tile').style.getPropertyValue('--y')).toBe('162');
    expect(find('.tiles').hasAttribute('data-flying')).toBe(false);
  });
  it('keeps a whole wide film letterboxed and its thumbnail placed by the camera', async () => {
    const film = { ...madeUpFilm(), size: { width: 16, height: 8 } };
    await show({ films: { ...empty, front: film } });
    const thumbnail = find('img');
    expect(thumbnail.style.width).toBe('208px');
    expect(thumbnail.style.height).toBe('104px');
    expect(thumbnail.style.top).toBe('156px');
    expect(thumbnail.hidden).toBe(false);
    tick(); expect(thumbnail.hidden).toBe(false);
    tick(); expect(thumbnail.hidden).toBe(true);
    expect(thumbnail.style.display).toBe('none');
    act(() => { for (const painter of painters) painter('lost'); });
    expect(thumbnail.hidden).toBe(false);
    await show({ films: { ...empty, front: { ...film, pixels: new Uint16Array(128) } } });
    expect(thumbnail.hidden).toBe(false);
  });
  it('glides the camera to a review point without restarting for the same framing', async () => {
    const film = { ...madeUpFilm(), size: { width: 586, height: 1300 } };
    await show({ moment: 'find', films: { ...empty, front: film } });
    const frame: StageFilm['frame'] = { kind: 'vertebra', centre: [293, 650], names: true, over: 2 };
    await show({ films: { ...empty, front: { ...film, frame } } });
    expect(cameras.isGliding('front')).toBe(true);
    tick(400);
    expect(cameras.zoomOverFitOf('front')).toBeCloseTo(1.5);
    await show({ films: { ...empty, front: { ...film, frame: { ...frame } } } });
    tick(400);
    expect(cameras.isGliding('front')).toBe(false);
    expect(cameras.zoomOverFitOf('front')).toBe(2);
    const placement = cameras.placementOf('front');
    if (placement === null) throw new Error('no camera');
    expect(filmSpace(placement).toScreen([293, 650])[1]).toBeCloseTo(322);
    expect(steps.size).toBe(0);
  });
  it('keeps the camera’s push-in gliding through the tile’s new layout box and back to whole', async () => {
    const film = { ...madeUpFilm(), size: { width: 586, height: 1300 } };
    await show({ films: { ...empty, front: film } });
    const frame: StageFilm['frame'] = { kind: 'spine', box: { left: 220, right: 366, top: 200, bottom: 1100 }, names: true, most: 1.3 };
    await show({ moment: 'find', films: { ...empty, front: { ...film, frame } } });
    expect(cameras.frameOf('front')?.viewport).toEqual({ width: 322, height: 644 });
    expect(cameras.isGliding('front')).toBe(true);
    tick(800);
    expect(cameras.zoomOverFitOf('front')).toBeCloseTo(1.3);
    await show({ moment: 'films', films: { ...empty, front: film } });
    expect(cameras.isGliding('front')).toBe(true);
    tick(800);
    expect(cameras.zoomOverFitOf('front')).toBe(1);
  });
  it('mounts the light before the outlines and forwards the outline pointer only in Find', async () => {
    const point = vi.fn();
    const film: StageFilm = { ...madeUpFilm(), light: { from: 0, to: 16, height: 4 }, outlines: [{ key: 'front:1', name: 'T1', spoken: 'T1', look: 'found', shown: true, quad: [[2, 2], [6, 2], [6, 4], [2, 4]] }] };
    const replay = { share: () => 0.5, subscribe: () => () => undefined };
    await show({ moment: 'find', films: { ...empty, front: film }, onPoint: point, replay });
    expect([...host.querySelectorAll('[data-layer]')].map((layer) => layer.getAttribute('data-layer'))).toEqual(['light', 'outlines']);
    expect(find('.travelling-light').getAttribute('y')).toBe('6');
    act(() => { find('.mark:not(.under) polygon').dispatchEvent(new MouseEvent('pointerover', { bubbles: true })); });
    expect(point).toHaveBeenCalledWith('front', 'front:1');
    point.mockClear();
    await show({ moment: 'films' });
    act(() => { find('.mark:not(.under) polygon').dispatchEvent(new MouseEvent('pointerover', { bubbles: true })); });
    expect(point).not.toHaveBeenCalled();
  });
});

it('keeps the Find registration for formation and exposes the same rail tiles for comparison', async () => {
  let driver: FilmDriver | null = null;
  const onFilmDriver = (next: FilmDriver | null) => { driver = next; };
  const current = (): FilmDriver => { if (driver === null) throw new Error('no film driver'); return driver; };
  const front: StageFilm = { ...madeUpFilm(), names: true, outlines: [{ key: 'T1', name: 'T1', spoken: 'T1', look: 'found', shown: true, quad: [[220, 200], [366, 200], [366, 220], [220, 220]] }], size: { width: 586, height: 1300 }, frame: { kind: 'spine', box: { left: 220, right: 366, top: 200, bottom: 1100 }, names: true, most: 1.3 } };
  await show({ moment: 'find', films: { ...empty, front, side: madeUpFilm() }, onFilmDriver });
  tick(800);
  const tile = find('.tile[data-place="front"]');
  const before = cameras.placementOf('front');
  if (before === null) throw new Error('no camera');
  const zoom = before.zoom;
  const openingBox = ['--x', '--y', '--bw', '--bh', '--s'].map((name) => tile.style.getPropertyValue(name));
  await show({ moment: 'three', forming: true });
  expect(['--x', '--y', '--bw', '--bh', '--s'].map((name) => tile.style.getPropertyValue(name))).toEqual(openingBox);
  expect(cameras.placementOf('front')).toEqual(before);
  expect(tile.style.opacity).not.toBe('0');
  expect(find('.tiles').hasAttribute('data-holding')).toBe(true);
  expect(tile.getAttribute('data-names')).toBe('on');
  expect(tile.querySelector('.mark.found polygon')).not.toBeNull();
  expect(find('.tile[data-place="front"]')).toBe(tile);
  expect(current().opening(DOMRect.fromRect())?.pxPerFilmPx).toBe(zoom);
  act(() => { current().set({ front: 0, side: 0, 'bend-left': 0, 'bend-right': 0 }); });
  expect(Number(tile.style.getPropertyValue('--s'))).toBeGreaterThan(1);
  act(() => { current().finish(); });
  expect(tile.style.getPropertyValue('--s')).toBe('1');
  await show({ forming: false, compared: 'front', onPressFilm: vi.fn() });
  expect(find('.tile[data-place="front"]')).toBe(tile);
  expect(current().compared).toBe('front');
  expect(host.querySelector('.compare-slot')).not.toBeNull();
  expect(current().separation).toBeGreaterThan(0);
});


it('does not render empty-place names during formation or the settled three moment', async () => {
  await show({ moment: 'find', films: { ...empty, front: madeUpFilm() } });
  for (const forming of [true, false]) {
    await show({ moment: 'three', forming });
    expect([...host.querySelectorAll('.caption')].map((caption) => caption.getAttribute('data-place'))).toEqual(['front']);
    for (const kind of ['side', 'bend-left', 'bend-right']) expect(host.querySelector(`.caption[data-place="${kind}"]`)).toBeNull();
  }
});
