import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import type { EngineClient } from './engine-client';
import { EngineProvider, useEngine } from './use-engine';

declare global {
  var IS_REACT_ACT_ENVIRONMENT: boolean | undefined;
}
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

const engine = { info: () => Promise.resolve({ engineVersion: 'x', shapesFingerprint: 'y', caseFormat: 'spine-case', caseVersion: 1 }) } as unknown as EngineClient;
let unmount: (() => void) | null = null;

afterEach(() => {
  act(() => {
    unmount?.();
  });
  unmount = null;
});

function render(element: React.ReactNode): void {
  const root = createRoot(document.createElement('div'));
  unmount = () => {
    root.unmount();
  };
  act(() => {
    root.render(element);
  });
}

describe('useEngine', () => {
  it('gives every part below the provider the engine it holds', () => {
    const seen: EngineClient[] = [];
    function Part() {
      seen.push(useEngine());
      return null;
    }
    render(
      <EngineProvider engine={engine}>
        <Part />
        <div>
          <Part />
        </div>
      </EngineProvider>,
    );
    expect(seen.length).toBeGreaterThanOrEqual(2);
    expect(seen.every((each) => each === engine)).toBe(true);
  });

  it('fails plainly when there is no provider', () => {
    function Part() {
      useEngine();
      return null;
    }
    expect(() => {
      render(<Part />);
    }).toThrow('useEngine is called outside an EngineProvider');
  });
});
