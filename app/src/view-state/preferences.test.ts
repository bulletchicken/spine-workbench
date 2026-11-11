import { afterEach, describe, expect, it, vi } from 'vitest';
import { MOTION_KEY, createPreferences, readMotionPreference, writeMotionPreference } from './preferences';

/** A stand-in for the browser's local storage, held in a map. */
function fakeStorage(initial: Record<string, string> = {}): Storage {
  const items = new Map(Object.entries(initial));
  return {
    get length() {
      return items.size;
    },
    clear: () => {
      items.clear();
    },
    getItem: (key: string) => items.get(key) ?? null,
    key: (index: number) => [...items.keys()][index] ?? null,
    removeItem: (key: string) => {
      items.delete(key);
    },
    setItem: (key: string, value: string) => {
      items.set(key, value);
    },
  };
}

/** Storage that refuses every use, as a blocked or full store does. */
function blockedStorage(): Storage {
  const refuse = (): never => {
    throw new DOMException('blocked', 'SecurityError');
  };
  return {
    length: 0,
    clear: refuse,
    getItem: refuse,
    key: refuse,
    removeItem: refuse,
    setItem: refuse,
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('reduced motion preference', () => {
  it('follows the system when nothing is remembered', () => {
    vi.stubGlobal('localStorage', fakeStorage());
    expect(readMotionPreference()).toBe('system');
  });

  it('reads back a remembered reduced choice', () => {
    vi.stubGlobal('localStorage', fakeStorage({ [MOTION_KEY]: 'reduced' }));
    expect(readMotionPreference()).toBe('reduced');
    writeMotionPreference('system');
    expect(readMotionPreference()).toBe('system');
  });

  it('ignores any stored value other than the word reduced', () => {
    for (const stored of ['REDUCED', '1', 'true', ' reduced', '']) {
      vi.stubGlobal('localStorage', fakeStorage({ [MOTION_KEY]: stored }));
      expect(readMotionPreference()).toBe('system');
    }
  });

  it('writes the choice under its own key', () => {
    const storage = fakeStorage();
    vi.stubGlobal('localStorage', storage);
    writeMotionPreference('reduced');
    expect(storage.getItem(MOTION_KEY)).toBe('reduced');
  });

  it('keeps working when storage refuses every use: system on read, the choice held in the store', () => {
    vi.stubGlobal('localStorage', blockedStorage());
    expect(readMotionPreference()).toBe('system');
    expect(() => {
      writeMotionPreference('reduced');
    }).not.toThrow();
    const store = createPreferences('system');
    expect(() => {
      store.getState().setMotion('reduced', true);
    }).not.toThrow();
    expect(store.getState().motion).toBe('reduced');
  });

  it('with no storage at all, as in a page without one, nothing is remembered and nothing fails', () => {
    expect(readMotionPreference()).toBe('system');
    expect(() => {
      writeMotionPreference('reduced');
    }).not.toThrow();
  });

  it('a store set to reduced remembers it, and starts from what was remembered', () => {
    const storage = fakeStorage();
    vi.stubGlobal('localStorage', storage);
    createPreferences('system').getState().setMotion('reduced', true);
    expect(storage.getItem(MOTION_KEY)).toBe('reduced');
    expect(createPreferences(readMotionPreference()).getState().motion).toBe('reduced');
  });

  it('a choice made for this visit only is not remembered, and the store still holds it', () => {
    const storage = fakeStorage();
    vi.stubGlobal('localStorage', storage);
    const store = createPreferences('system');
    store.getState().setMotion('reduced', false);
    expect(store.getState().motion).toBe('reduced');
    expect(storage.getItem(MOTION_KEY)).toBeNull();
    expect(readMotionPreference()).toBe('system');
  });
});
