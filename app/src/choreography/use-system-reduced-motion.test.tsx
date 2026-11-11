// The hook against a system setting the test flips by hand. The test
// page has no media queries of its own, so `matchMedia` is a stand-in
// that answers for one query and tells its listeners when it changes.

import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SYSTEM_ASKS_FOR_REDUCED } from './reduced-motion';
import { useSystemReducedMotion } from './use-system-reduced-motion';

/** A system setting: what it is now, who is listening, and a way to change it. */
function handSetting(asksForReduced: boolean) {
  const listeners = new Set<() => void>();
  let asks = asksForReduced;
  const asked: string[] = [];
  const matchMedia = (query: string) => {
    asked.push(query);
    return {
      get matches() {
        return asks;
      },
      addEventListener: (_: 'change', listener: () => void) => listeners.add(listener),
      removeEventListener: (_: 'change', listener: () => void) => listeners.delete(listener),
    };
  };
  return {
    matchMedia,
    asked,
    listening: () => listeners.size,
    set: (next: boolean) => {
      asks = next;
      for (const listener of [...listeners]) listener();
    },
  };
}

function Shows() {
  return <p>{useSystemReducedMotion() ? 'reduced' : 'full'}</p>;
}

describe('useSystemReducedMotion', () => {
  let host: HTMLElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
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

  it('answers with the system’s setting as it stands', () => {
    const setting = handSetting(true);
    vi.stubGlobal('matchMedia', setting.matchMedia);
    act(() => {
      root.render(<Shows />);
    });
    expect(host.textContent).toBe('reduced');
    expect(new Set(setting.asked)).toEqual(new Set([SYSTEM_ASKS_FOR_REDUCED]));
  });

  it('follows the setting when it changes, both ways', () => {
    const setting = handSetting(false);
    vi.stubGlobal('matchMedia', setting.matchMedia);
    act(() => {
      root.render(<Shows />);
    });
    expect(host.textContent).toBe('full');

    act(() => {
      setting.set(true);
    });
    expect(host.textContent).toBe('reduced');

    act(() => {
      setting.set(false);
    });
    expect(host.textContent).toBe('full');
  });

  it('stops listening when the part that asked is gone', () => {
    const setting = handSetting(false);
    vi.stubGlobal('matchMedia', setting.matchMedia);
    act(() => {
      root.render(<Shows />);
    });
    expect(setting.listening()).toBe(1);

    act(() => {
      root.render(null);
    });
    expect(setting.listening()).toBe(0);
  });
});
