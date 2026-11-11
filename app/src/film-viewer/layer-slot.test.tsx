import { act, Profiler, useEffect } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { filmSpace, type FilmHud, type FilmHudProps, type FilmLayer, type FilmLayerProps } from './film-space';
import { LayerStack } from './LayerStack';
import { useHud, useLayer } from './layer-slot';

declare global {
  // Tells React it is running inside a test, so act() is expected.
  var IS_REACT_ACT_ENVIRONMENT: boolean | undefined;
}
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

const space = filmSpace({ size: { width: 100, height: 200 }, view: { quarterTurns: 0, mirrored: false }, zoom: 2, pan: [10, 20] });

/** A layer with a focusable dot, like a pin. */
function Dots({ n, space: given }: { n: number } & FilmLayerProps) {
  return (
    <circle className="dot" data-n={n} r={given.screenToFilmLength(4)} cx={n} cy={n} tabIndex={0}>
      <title>{`dot ${String(n)}`}</title>
    </circle>
  );
}

function Made({ n, onLayer }: { n: number; onLayer?: (layer: FilmLayer) => void }) {
  const layer = useLayer('dots', Dots, { n });
  useEffect(() => {
    onLayer?.(layer);
  });
  return <LayerStack space={space} layers={[layer]} />;
}

/** The mistake useLayer exists to prevent: a Draw made afresh on each render. */
function Naive({ n }: { n: number }) {
  const Draw = (props: FilmLayerProps) => <Dots n={n} {...props} />;
  return <LayerStack space={space} layers={[{ id: 'dots', Draw }]} />;
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

function dot(): SVGCircleElement {
  const found = host.querySelector<SVGCircleElement>('circle.dot');
  if (found === null) throw new Error('no dot');
  return found;
}

describe('a layer made with useLayer', () => {
  it('keeps its DOM nodes, and focus, through ten changes of props', () => {
    const seen = new Set<FilmLayer>();
    const keep = (layer: FilmLayer) => seen.add(layer);
    show(<Made n={0} onLayer={keep} />);
    const first = dot();
    first.focus();
    expect(document.activeElement).toBe(first);
    for (let n = 1; n <= 10; n += 1) {
      show(<Made n={n} onLayer={keep} />);
      expect(dot()).toBe(first);
      expect(dot().getAttribute('data-n')).toBe(String(n));
      expect(document.activeElement).toBe(first);
    }
    expect(seen.size).toBe(1);
  });

  it('sits in a group set to the film’s matrix', () => {
    show(<Made n={0} />);
    expect(dot().parentElement?.getAttribute('transform')).toBe('matrix(2 0 0 2 11 21)');
    expect(dot().getAttribute('r')).toBe('2');
  });

  it('is needed: a Draw made on every render loses its nodes and focus', () => {
    show(<Naive n={0} />);
    const first = dot();
    first.focus();
    show(<Naive n={1} />);
    expect(dot()).not.toBe(first);
    expect(document.activeElement).not.toBe(first);
  });

  it('re-renders its part only when a field of its props changes', () => {
    function Label({ label }: { label: string } & FilmLayerProps) {
      return <text>{label}</text>;
    }
    let made: FilmLayer | undefined;
    function Parent({ label, tick }: { label: string; tick: number }) {
      // A fresh props object on every render, equal field by field while the label stays.
      const layer = useLayer('label', Label, { label, tick: tick >= 0 ? 'same' : 'never' });
      useEffect(() => {
        made = layer;
      });
      return null;
    }
    show(<Parent label="T8" tick={0} />);
    if (made === undefined) throw new Error('no layer');
    const { Draw } = made;
    // The Draw is mounted where the parent's renders do not reach, so only the slot can re-render it.
    const elsewhere = document.createElement('div');
    document.body.append(elsewhere);
    const other = createRoot(elsewhere);
    let commits = 0;
    const counted = () => {
      commits += 1;
    };
    act(() => {
      other.render(
        <Profiler id="label" onRender={counted}>
          <Draw space={space} />
        </Profiler>,
      );
    });
    const afterMount = commits;
    show(<Parent label="T8" tick={1} />);
    show(<Parent label="T8" tick={2} />);
    expect(commits).toBe(afterMount);
    show(<Parent label="T9" tick={3} />);
    expect(commits).toBe(afterMount + 1);
    expect(elsewhere.textContent).toBe('T9');
    act(() => {
      other.unmount();
    });
    elsewhere.remove();
  });
});

describe('the layer stack', () => {
  it('draws the layers in the order given, first at the bottom, and the tool’s preview above them all', () => {
    const Nothing = () => null;
    const layers: FilmLayer[] = [
      { id: 'outlines', Draw: Nothing },
      { id: 'marks', Draw: Nothing },
      { id: 'labels', Draw: Nothing },
    ];
    show(<LayerStack space={space} layers={layers} preview={<circle r={1} />} />);
    const order = [...host.querySelectorAll('svg > g')].map((group) => group.getAttribute('data-layer'));
    expect(order).toEqual(['outlines', 'marks', 'labels', 'tool-preview']);
  });

  it('does not re-render a layer when the stack renders again with the same space', () => {
    const renders: number[] = [];
    function Counted({ n }: { n: number } & FilmLayerProps) {
      renders.push(n);
      return <circle className="dot" r={n} />;
    }
    function Stack({ hover }: { hover: number }) {
      const layer = useLayer('counted', Counted, { n: 1 });
      return <LayerStack space={space} layers={[layer]} preview={<circle r={hover} />} />;
    }
    show(<Stack hover={0} />);
    const afterMount = renders.length;
    for (let hover = 1; hover <= 10; hover += 1) show(<Stack hover={hover} />);
    expect(renders).toHaveLength(afterMount);
  });
});

describe('a HUD made with useHud', () => {
  it('keeps its identity and its nodes while its props change', () => {
    function Badge({ word, viewport }: { word: string } & FilmHudProps) {
      return (
        <button type="button" className="badge" data-width={viewport.width}>
          {word}
        </button>
      );
    }
    const seen = new Set<FilmHud>();
    function Parent({ word }: { word: string }) {
      const hud = useHud('badge', Badge, { word });
      useEffect(() => {
        seen.add(hud);
      });
      return <hud.Draw space={space} viewport={{ width: 320, height: 700 }} />;
    }
    show(<Parent word="L" />);
    const button = host.querySelector('button');
    button?.focus();
    for (const word of ['R', 'L', 'R']) show(<Parent word={word} />);
    expect(host.querySelector('button')).toBe(button);
    expect(button?.textContent).toBe('R');
    expect(button?.getAttribute('data-width')).toBe('320');
    expect(document.activeElement).toBe(button);
    expect(seen.size).toBe(1);
  });
});
