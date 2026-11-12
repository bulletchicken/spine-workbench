import { act, createRef } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActiveTool, ToolPointer, ToolView } from '@/toolbox/tool-contract';
import type { CameraIntent } from './camera/gestures';
import type { FilmLayer, FilmLayerProps, FilmPlacement } from './film-space';
import { FilmViewer, type FilmViewerHandle } from './FilmViewer';
import type { FilmLook } from './gpu/film-shader';
import { useLayer } from './layer-slot';
import { toolChanged } from './tool-changes';

declare global {
  // Tells React it is running inside a test, so act() is expected.
  var IS_REACT_ACT_ENVIRONMENT: boolean | undefined;
}
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

// jsdom has no layout, no GPU, no pointer capture and no media queries.
// The viewer measures itself as 400 × 800 here, draws nothing, captures
// nothing, and hears of a new pixel ratio only when a test says so.
class StillResizeObserver {
  static viewers: { element: Element; notify: ResizeObserverCallback }[] = [];
  constructor(private readonly notify: ResizeObserverCallback) {}
  observe(element: Element) {
    StillResizeObserver.viewers.push({ element, notify: this.notify });
  }
  disconnect() {
    return undefined;
  }
}

/** The resolution queries the viewer is listening to, which a test can tell the ratio changed. */
const resolutionQueries = new Set<EventTarget>();

function pixelRatioBecomes(ratio: number) {
  vi.stubGlobal('devicePixelRatio', ratio);
  act(() => {
    for (const query of [...resolutionQueries]) query.dispatchEvent(new Event('change'));
  });
}

beforeEach(() => {
  StillResizeObserver.viewers = [];
  resolutionQueries.clear();
  vi.stubGlobal('devicePixelRatio', 1);
  vi.stubGlobal('matchMedia', () => {
    const query = new EventTarget();
    const add = query.addEventListener.bind(query);
    const remove = query.removeEventListener.bind(query);
    return Object.assign(query, {
      addEventListener: (type: string, listener: EventListener) => {
        resolutionQueries.add(query);
        add(type, listener);
      },
      removeEventListener: (type: string, listener: EventListener) => {
        resolutionQueries.delete(query);
        remove(type, listener);
      },
    });
  });
  vi.stubGlobal('ResizeObserver', StillResizeObserver);
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(400);
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue(DOMRect.fromRect({ x: 0, y: 0, width: 400, height: 800 }));
  Object.assign(HTMLElement.prototype, {
    setPointerCapture: () => undefined,
    releasePointerCapture: () => undefined,
    hasPointerCapture: () => false,
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

/** A 100 × 200 film at zoom 2, its outer corner at (10, 20). Film pixel (x, y) is on screen at (11 + 2x, 21 + 2y). */
const placement: FilmPlacement = { size: { width: 100, height: 200 }, view: { quarterTurns: 0, mirrored: false }, zoom: 2, pan: [10, 20] };

/** A tool that writes down what it heard and shows the cursor it is told to. */
function listeningTool(cursor = 'crosshair') {
  const heard: [string, ToolPointer | null][] = [];
  const view: ToolView = { cursor, hint: null, preview: null };
  const tool: ActiveTool = {
    pointerDown: (p) => heard.push(['down', p]),
    pointerMove: (p) => heard.push(['move', p]),
    pointerUp: (p) => heard.push(['up', p]),
    pointerLeave: () => heard.push(['leave', null]),
    escape: () => heard.push(['escape', null]),
    view: () => view,
    stop: () => undefined,
  };
  return { tool, heard, view };
}

let host: HTMLDivElement;
let root: Root;

beforeEach(() => {
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});

afterEach(() => {
  act(() => {
    root.unmount();
  });
  host.remove();
});

function show(node: React.ReactNode) {
  act(() => {
    root.render(node);
  });
}

function viewer(): HTMLElement {
  const found = host.querySelector<HTMLElement>('.film-viewer');
  if (found === null) throw new Error('no viewer');
  return found;
}

function pointer(type: string, x: number, y: number, extra: { button?: number; target?: Element } = {}) {
  act(() => {
    (extra.target ?? viewer()).dispatchEvent(
      new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y, button: extra.button ?? 0 }),
    );
  });
}

describe('the film viewer', () => {
  it('measures its layout size, not its transformed box', () => {
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue(DOMRect.fromRect({ x: 20, y: 30, width: 200, height: 400 }));
    const told: { width: number; height: number }[] = [];
    show(<FilmViewer pixels={null} placement={placement} label="Front film" onViewport={(box) => told.push(box)} />);
    expect(told).toEqual([{ width: 400, height: 800 }]);
    const observation = StillResizeObserver.viewers.find(({ element }) => element === viewer());
    if (observation === undefined) throw new Error('no viewport observer');
    act(() => {
      observation.notify([{ target: viewer(), contentRect: DOMRect.fromRect({ width: 322, height: 644 }), borderBoxSize: [], contentBoxSize: [], devicePixelContentBoxSize: [] }], {} as ResizeObserver);
    });
    expect(told.at(-1)).toEqual({ width: 322, height: 644 });
    expect([host.querySelector('canvas')?.width, host.querySelector('canvas')?.height]).toEqual([322, 644]);
  });
  it('hands a click to the tool in film pixels, as a press and a release', () => {
    const { tool, heard } = listeningTool();
    show(<FilmViewer pixels={null} placement={placement} tool={tool} label="Front film" />);
    pointer('pointerdown', 31, 61);
    expect(heard).toEqual([]);
    pointer('pointerup', 33, 62);
    expect(heard.map(([what]) => what)).toEqual(['down', 'up']);
    expect(heard[0]?.[1]?.at).toEqual([10, 20]);
    expect(heard[0]?.[1]?.onFilm).toBe(true);
    expect(heard[1]?.[1]?.at).toEqual([11, 20.5]);
  });

  it('counts the film’s outer half-pixel border as on the film, and no further', () => {
    const { tool, heard } = listeningTool();
    show(<FilmViewer pixels={null} placement={placement} tool={tool} label="Front film" />);
    // Film (x, y) is on screen at (11 + 2x, 21 + 2y): the outer edges are at x = −0.5 and 99.5, y = −0.5 and 199.5.
    pointer('pointermove', 10.5, 20.5);
    pointer('pointermove', 209.5, 419.5);
    pointer('pointermove', 9.5, 100);
    pointer('pointermove', 100, 420.5);
    expect(heard.map(([, p]) => p?.at)).toEqual([
      [-0.25, -0.25],
      [99.25, 199.25],
      [-0.75, 39.5],
      [44.5, 199.75],
    ]);
    expect(heard.map(([, p]) => p?.onFilm)).toEqual([true, true, false, false]);
  });

  it('pans with a drag past four pixels, and the tool never hears of it', () => {
    const { tool, heard } = listeningTool();
    const intents: CameraIntent[] = [];
    show(<FilmViewer pixels={null} placement={placement} tool={tool} onCamera={(intent) => intents.push(intent)} label="Front film" />);
    pointer('pointerdown', 100, 100);
    pointer('pointermove', 103, 100);
    pointer('pointermove', 106, 98);
    pointer('pointermove', 116, 90);
    pointer('pointerup', 116, 90);
    expect(intents).toEqual([
      { kind: 'pan', by: [6, -2] },
      { kind: 'pan', by: [10, -8] },
    ]);
    expect(heard).toEqual([]);
  });

  it('gives a press to the tool at once where its cursor says something can be grabbed', () => {
    const { tool, heard } = listeningTool('grab');
    const intents: CameraIntent[] = [];
    show(<FilmViewer pixels={null} placement={placement} tool={tool} onCamera={(intent) => intents.push(intent)} label="Front film" />);
    pointer('pointerdown', 11, 21);
    pointer('pointermove', 41, 61);
    pointer('pointerup', 41, 61);
    expect(heard.map(([what]) => what)).toEqual(['down', 'move', 'up']);
    expect(heard[1]?.[1]?.at).toEqual([15, 20]);
    expect(intents).toEqual([]);
  });

  it('tells the tool to let go when the browser takes away a press it holds', () => {
    const { tool, heard } = listeningTool('grab');
    show(<FilmViewer pixels={null} placement={placement} tool={tool} label="Front film" />);
    pointer('pointerdown', 11, 21);
    pointer('pointercancel', 11, 21);
    pointer('pointermove', 41, 61);
    expect(heard.map(([what]) => what)).toEqual(['down', 'escape', 'move']);
  });

  it('tells the tool where the pointer hovers, and when it leaves', () => {
    const { tool, heard } = listeningTool();
    show(<FilmViewer pixels={null} placement={placement} tool={tool} label="Front film" />);
    pointer('pointermove', 9, 21);
    // React sees a pointer leave as a pointerout to somewhere outside.
    act(() => {
      viewer().dispatchEvent(new MouseEvent('pointerout', { bubbles: true, relatedTarget: document.body }));
    });
    expect(heard[0]?.[1]?.at).toEqual([-1, 0]);
    expect(heard[0]?.[1]?.onFilm).toBe(false);
    expect(heard[1]?.[0]).toBe('leave');
  });

  it('leaves a press on a button drawn over the film to that button', () => {
    function Ring({ space }: FilmLayerProps) {
      return <circle className="ring" role="button" r={space.screenToFilmLength(10)} />;
    }
    const layers: FilmLayer[] = [{ id: 'ring', Draw: Ring }];
    const { tool, heard } = listeningTool();
    show(<FilmViewer pixels={null} placement={placement} tool={tool} layers={layers} label="Front film" />);
    const ring = host.querySelector('.ring');
    if (ring === null) throw new Error('no ring');
    pointer('pointerdown', 50, 50, { target: ring });
    pointer('pointerup', 50, 50, { target: ring });
    expect(heard.filter(([what]) => what === 'down')).toEqual([]);
  });

  it('takes the right button for brightness and contrast, whatever the tool', () => {
    const { tool, heard } = listeningTool('grab');
    const looks: FilmLook[] = [];
    show(<FilmViewer pixels={null} placement={placement} tool={tool} onLookChange={(look) => looks.push(look)} label="Front film" />);
    pointer('pointerdown', 200, 400, { button: 2 });
    pointer('pointermove', 300, 400);
    pointer('pointerup', 300, 400, { button: 2 });
    // A quarter of the 400-pixel width to the right widens the window by a quarter.
    expect(looks.at(-1)).toEqual({ centre: 0.5, width: 1.25, flipped: false });
    expect(heard).toEqual([]);
  });

  it('fits on a double-click and zooms at the pointer on the wheel', () => {
    const intents: CameraIntent[] = [];
    show(<FilmViewer pixels={null} placement={placement} onCamera={(intent) => intents.push(intent)} label="Front film" />);
    act(() => {
      viewer().dispatchEvent(new MouseEvent('dblclick', { bubbles: true, clientX: 10, clientY: 10 }));
      viewer().dispatchEvent(new WheelEvent('wheel', { bubbles: true, cancelable: true, deltaY: -100, clientX: 120, clientY: 240 }));
    });
    expect(intents[0]).toEqual({ kind: 'fit' });
    expect(intents[1]?.kind).toBe('zoom');
    expect(intents[1]?.kind === 'zoom' ? intents[1].at : null).toEqual([120, 240]);
    expect(intents[1]?.kind === 'zoom' ? intents[1].factor : 0).toBeCloseTo(1.16, 12);
  });

  it('moves the layers with `place` at once, and hands them the new space when the camera lands', () => {
    const spaces: number[] = [];
    function Probe({ space }: FilmLayerProps) {
      spaces.push(space.placement.zoom);
      return <circle className="probe" r={1} />;
    }
    const layers: FilmLayer[] = [{ id: 'probe', Draw: Probe }];
    const handle = createRef<FilmViewerHandle>();
    show(<FilmViewer ref={handle} pixels={null} placement={placement} layers={layers} label="Front film" />);
    const group = () => host.querySelector('.probe')?.parentElement?.getAttribute('transform');
    expect(group()).toBe('matrix(2 0 0 2 11 21)');
    const rendersBefore = spaces.length;
    act(() => {
      handle.current?.place({ ...placement, zoom: 4 }, true);
    });
    expect(group()).toBe('matrix(4 0 0 4 12 22)');
    expect(spaces.length).toBe(rendersBefore);
    act(() => {
      handle.current?.place({ ...placement, zoom: 3 }, false);
    });
    expect(group()).toBe('matrix(3 0 0 3 11.5 21.5)');
    expect(spaces.at(-1)).toBe(3);
  });

  it('moves the film’s glow with `place`, cut to the viewport and the glow’s room past it', () => {
    const handle = createRef<FilmViewerHandle>();
    show(<FilmViewer ref={handle} pixels={null} placement={placement} label="Front film" />);
    const glow = () => {
      const style = host.querySelector<HTMLElement>('.film-viewer__glow')?.style;
      return [style?.left, style?.top, style?.width, style?.height];
    };
    // At zoom 2 from (10, 20) the 100 × 200 film covers 200 × 400.
    expect(glow()).toEqual(['10px', '20px', '200px', '400px']);
    // At zoom 10 from (−300, −500) it covers (−300, −500) to (700, 1500); the
    // 400 × 800 viewport plus 160 each way cuts that to (−160, −160) to (560, 960).
    act(() => {
      handle.current?.place({ ...placement, zoom: 10, pan: [-300, -500] }, true);
    });
    expect(glow()).toEqual(['-160px', '-160px', '720px', '1120px']);
  });

  it('writes the zoom of every placement, glides included, where CSS can size marks from it', () => {
    const handle = createRef<FilmViewerHandle>();
    show(<FilmViewer ref={handle} pixels={null} placement={placement} label="Front film" />);
    const zoom = () => viewer().style.getPropertyValue('--film-zoom');
    expect(zoom()).toBe('2');
    act(() => {
      handle.current?.place({ ...placement, zoom: 3.5 }, true);
    });
    expect(zoom()).toBe('3.5');
    act(() => {
      handle.current?.place({ ...placement, zoom: 5 }, false);
    });
    expect(zoom()).toBe('5');
  });

  it('does not re-render the layers when the tool changes or the right button drags', () => {
    const renders: number[] = [];
    function Counted({ n }: { n: number } & FilmLayerProps) {
      renders.push(n);
      return <circle r={n} />;
    }
    const { tool } = listeningTool();
    function Viewer() {
      const layer = useLayer('counted', Counted, { n: 1 });
      return <FilmViewer pixels={null} placement={placement} tool={tool} layers={[layer]} label="Front film" />;
    }
    show(<Viewer />);
    const afterMount = renders.length;
    for (let i = 0; i < 10; i += 1) {
      act(() => {
        toolChanged(tool);
      });
    }
    pointer('pointerdown', 200, 400, { button: 2 });
    pointer('pointermove', 260, 380);
    pointer('pointerup', 260, 380, { button: 2 });
    expect(renders).toHaveLength(afterMount);
  });

  it('sizes its canvas again when the device pixel ratio changes', () => {
    show(<FilmViewer pixels={null} placement={placement} label="Front film" />);
    const canvas = host.querySelector('canvas');
    expect([canvas?.width, canvas?.height]).toEqual([400, 800]);
    pixelRatioBecomes(2);
    expect([canvas?.width, canvas?.height]).toEqual([800, 1600]);
  });

  it('tells its size once, however often it is handed a new callback', () => {
    const told: { width: number; height: number }[] = [];
    for (let i = 0; i < 4; i += 1) {
      show(<FilmViewer pixels={null} placement={placement} onViewport={(viewport) => told.push(viewport)} label="Front film" />);
    }
    expect(told).toEqual([{ width: 400, height: 800 }]);
  });

  it('reads the tool’s view again when the tool says it changed', () => {
    const { tool, view } = listeningTool('crosshair');
    show(<FilmViewer pixels={null} placement={placement} tool={tool} label="Front film" />);
    expect(viewer().style.cursor).toBe('crosshair');
    view.cursor = 'grab';
    act(() => {
      toolChanged(tool);
    });
    expect(viewer().style.cursor).toBe('grab');
  });
});
