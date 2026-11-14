import { act, createRef } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Level } from '@/shapes';
import type { Bone } from '../bones/read-bones';
import type { ViewOrRule } from '../scene/stage';
import { FRONT_ONLY_CASE, MADE_UP_CASE } from '../made-up-case/made-up-case';
import { fittedFilmCamera } from '../placing/fitted-film-camera';
import type { SpineModel } from '../scene/spine-model';
import type { Pointed } from '../scene/pointed';
import type { SceneLayer } from '../scene/scene-layer';
import { Spine3D, type Spine3DHandle } from './Spine3D';

declare global {
  // Tells React it is running inside a test, so act() is expected.
  var IS_REACT_ACT_ENVIRONMENT: boolean | undefined;
}
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

// jsdom has no GPU, so the stage is stood in for by one that writes down
// what it is asked to do. Everything up to the stage is real: each bone (a
// plain block here) is shaped to its fitted vertebra and given its shapes.
const heard = vi.hoisted(() => ({ calls: [] as [string, ...unknown[]][], models: [] as unknown[], hit: null as Pointed | null }));

vi.mock('../scene/stage', () => ({
  makeStage: () => {
    const say = (name: string, ...more: unknown[]) => {
      heard.calls.push([name, ...more]);
    };
    const films = new Map<string, unknown>();
    const view = { azimuth: 0, elevation: 0, perspective: 0, pxPerMm: [1, 1], aim: [0, 0, 0], aimAt: [0, 0] };
    let viewsSet = 0;
    return {
      addSpine: (model: { bones: { level: string }[] }) => {
        heard.models.push(model);
        say('addSpine', model.bones.length);
        return {
          levels: model.bones.map((bone) => bone.level),
          set: (values: unknown, levels?: unknown) => {
            say('spine.set', values, levels);
          },
          setSpine: (values: unknown) => {
            say('spine.setSpine', values);
          },
          pressOnto: (plane: unknown) => {
            say('spine.pressOnto', plane);
          },
          settle: () => {
            say('spine.settle');
          },
          remove: () => {
            say('spine.remove');
          },
        };
      },
      hangFilm: (name: string, camera: unknown, pixels: unknown) => {
        say('hangFilm', name, camera, pixels);
        const wall = {
          name: `the ${name} film`,
          remove: () => {
            films.delete(name);
            say('film.remove', name);
          },
        };
        films.set(name, wall);
        return wall;
      },
      film: (name: string) => films.get(name) ?? null,
      setView: (next: ViewOrRule) => {
        viewsSet += 1;
        say('setView', typeof next === 'function' ? next({ width: 400, height: 600 }) : next);
      },
      view: () => view,
      viewsSet: () => viewsSet,
      namedView: (name: string) => ({ ...view, named: name }),
      size: () => ({ width: 400, height: 600 }),
      bounds: () => ({ low: [0, 0, 0], high: [1, 1, 1] }),
      pick: () => heard.hit,
      onDrawn: () => () => undefined,
      dispose: () => {
        say('dispose');
      },
    };
  },
}));

const { fit, front, side } = MADE_UP_CASE;
const SIDE_PLACEMENT = fit.sideFilm;
if (SIDE_PLACEMENT === null) throw new Error('the pair has no side film placement');

/** A plain block standing in for a bone: 20 mm deep, 30 wide and 20 tall, its two plates level. */
function block(level: Level): Bone {
  const points: number[] = [];
  for (const x of [-10, 10]) for (const y of [-15, 15]) for (const z of [-10, 10]) points.push(x, y, z);
  // Two triangles a face, each wound to face outwards.
  const triangles = [0, 1, 3, 0, 3, 2, 4, 6, 7, 4, 7, 5, 0, 4, 5, 0, 5, 1, 2, 3, 7, 2, 7, 6, 0, 2, 6, 0, 6, 4, 1, 5, 7, 1, 7, 3];
  const plate = (z: number) => ({ centre: [0, 0, z] as [number, number, number], halfWidth: 15, halfDepth: 10 });
  return {
    level,
    source: 'a block',
    height: 20,
    halfWidth: 15,
    halfDepth: 10,
    body: { top: plate(10), bottom: plate(-10), wedgeFront: 0, wedgeSide: 0 },
    referenceCentre: [0, 0, 0],
    referenceAxes: [
      [1, 0, 0],
      [0, 1, 0],
      [0, 0, 1],
    ],
    pointCount: 8,
    triangleCount: 12,
    pointsAt: 0,
    trianglesAt: 0,
    points: new Float32Array(points),
    triangles: new Uint16Array(triangles),
  };
}

const bones = fit.vertebrae.map((vertebra) => block(vertebra.level));

let host: HTMLDivElement;
let root: Root;

beforeEach(() => {
  heard.calls.length = 0;
  heard.models.length = 0; heard.hit = null;
  vi.stubGlobal('matchMedia', () => ({ matches: true }));
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});

afterEach(() => {
  act(() => {
    root.unmount();
  });
  host.remove();
  vi.unstubAllGlobals();
});

function show(node: React.ReactNode) {
  act(() => {
    root.render(node);
  });
}

const named = (name: string) => heard.calls.filter(([call]) => call === name);

describe('Spine3D', () => {
  it('is a canvas a screen reader can name', () => {
    show(<Spine3D fit={fit} bones={bones} />);
    const canvas = host.querySelector('canvas');
    expect(canvas?.getAttribute('role')).toBe('img');
    expect(canvas?.getAttribute('aria-label')).toBe('The spine in 3D');
    show(<Spine3D fit={fit} bones={bones} label="The spine, from the front" />);
    expect(host.querySelector('canvas')?.getAttribute('aria-label')).toBe('The spine, from the front');
  });

  it('stands one spine, shaped from the bones to the fit, with its shadow, in the view it opens in', () => {
    show(<Spine3D fit={fit} bones={bones} />);
    expect(named('addSpine')).toEqual([['addSpine', 17]]);
    expect(named('spine.setSpine')).toContainEqual(['spine.setSpine', { shadow: 1 }]);
    // The opening view is set at once, not travelled to.
    expect(named('setView')).toEqual([['setView', expect.objectContaining({ named: 'three-quarter' })]]);
    const model = heard.models[0] as SpineModel;
    expect(model.bones.map((bone) => bone.level)).toEqual(fit.vertebrae.map((vertebra) => vertebra.level));
    expect(model.missing).toEqual([]);
  });

  it('hangs the films it is given, each from its own camera, and lays "flat" on the front film', () => {
    const films = { front: { geometry: front.geometry, size: front.size, pixels: null }, side: { geometry: side.geometry, size: side.size, pixels: null } };
    show(<Spine3D fit={fit} bones={bones} films={films} />);
    expect(named('hangFilm')).toEqual([
      ['hangFilm', 'front', fittedFilmCamera(front.geometry, front.size, fit.frontFilm), null],
      ['hangFilm', 'side', fittedFilmCamera(side.geometry, side.size, SIDE_PLACEMENT), null],
    ]);
    expect(named('spine.pressOnto').at(-1)).toEqual(['spine.pressOnto', expect.objectContaining({ name: 'the front film' })]);
  });

  it('works with the front film alone, as a real case has, and with no film at all', () => {
    show(<Spine3D fit={FRONT_ONLY_CASE.fit} bones={bones} films={{ front: { geometry: FRONT_ONLY_CASE.front.geometry, size: FRONT_ONLY_CASE.front.size, pixels: null } }} />);
    expect(named('hangFilm').map(([, name]) => name)).toEqual(['front']);
    show(<Spine3D fit={fit} bones={bones} />);
    expect(named('film.remove')).toEqual([['film.remove', 'front']]);
    expect(named('spine.pressOnto').at(-1)).toEqual(['spine.pressOnto', null]);
  });

  it('rehangs films when the fit changes, using its scale even when the film has a stated one', () => {
    const films = { front: { ...front, pixels: null }, side: { ...side, pixels: null } };
    show(<Spine3D fit={fit} bones={bones} films={films} />);
    const changed = { ...fit, frontFilm: { facingAssumed: false, mmPerPx: [0.7, 0.8] as [number, number], originPx: [123, 456] as [number, number] }, sideFilm: null };
    show(<Spine3D fit={changed} bones={bones} films={films} />);
    expect(named('hangFilm').at(-1)).toEqual(['hangFilm', 'front', expect.objectContaining({ uprightMmPerPx: [0.7, 0.8], origin: [123, 456] }), null]);
    expect(named('hangFilm').filter(([, name]) => name === 'side')).toHaveLength(1);
  });

  it('tints the levels it is told to, and no others', () => {
    const across: Level[] = ['T6', 'T7', 'T8'];
    show(<Spine3D fit={fit} bones={bones} highlighted={across} />);
    expect(named('spine.set').slice(-2)).toEqual([
      ['spine.set', { tint: 0 }, undefined],
      ['spine.set', { tint: 1 }, across],
    ]);
  });

  it('jumps to a new view when motion is reduced, and hands out the stage and the spine to be driven', () => {
    const handle = createRef<Spine3DHandle>();
    const ready = vi.fn();
    show(<Spine3D fit={fit} bones={bones} ref={handle} onReady={ready} />);
    expect(ready).toHaveBeenCalledTimes(1);
    expect(handle.current?.stage()).not.toBeNull();
    expect(handle.current?.spine()?.levels.length).toBe(17);
    show(<Spine3D fit={fit} bones={bones} view="front" ref={handle} onReady={ready} />);
    expect(named('setView').at(-1)).toEqual(['setView', expect.objectContaining({ named: 'front' })]);
    act(() => {
      handle.current?.settle();
    });
    expect(named('spine.settle').length).toBe(1);
    act(()=>{handle.current?.addFlatSpine();handle.current?.addSpine();});
    expect(heard.models).toHaveLength(3);
    expect(heard.models[1]).toBe(heard.models[0]);expect(heard.models[2]).toBe(heard.models[0]);
  });

  it('gives everything back when it goes: the stage, the spine and the films', () => {
    const ready = vi.fn();
    show(<Spine3D fit={fit} bones={bones} films={{ front: { geometry: front.geometry, size: front.size, pixels: null } }} onReady={ready} />);
    act(() => {
      root.unmount();
    });
    root = createRoot(host);
    const after = heard.calls.map(([call]) => call);
    expect(after.filter((call) => call === 'dispose')).toEqual(['dispose']);
    expect(after).toContain('spine.remove');
    expect(after).toContain('film.remove');
    expect(ready).toHaveBeenLastCalledWith(null);
    expect(host.querySelector('canvas')?.hasAttribute('data-hand')).not.toBe(true);
  });
});


it('passes a layer identity through the rich callback while preserving level-only screen callbacks', () => {
  const handle = createRef<Spine3DHandle>(), onPoint = vi.fn(), onPointed = vi.fn();
  show(<Spine3D fit={fit} bones={bones} ref={handle} onPoint={onPoint} onPointed={onPointed} />);
  const spine = handle.current?.spine(), canvas = host.querySelector('canvas');
  if (spine == null || canvas === null) throw new Error('Missing drawing');
  const part: Pointed = { kind: 'part', spine, layer: {} as SceneLayer, id: 'anchor:T8' };
  heard.hit = part;
  act(() => { canvas.dispatchEvent(new MouseEvent('pointermove', { clientX: 30, clientY: 40 })); });
  expect(onPointed).toHaveBeenLastCalledWith(part); expect(onPoint).toHaveBeenLastCalledWith(null);
  heard.hit = { kind: 'bone', spine, level: 'T8' };
  act(() => { canvas.dispatchEvent(new MouseEvent('pointermove', { clientX: 31, clientY: 40 })); });
  expect(onPoint).toHaveBeenLastCalledWith('T8');
  expect(named('spine.set').at(-1)).toEqual(['spine.set', { tint: 1 }, 'T8']);
});
