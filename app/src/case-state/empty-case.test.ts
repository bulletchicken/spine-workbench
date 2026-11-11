import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { APP_VERSION, emptyCase } from './empty-case';

describe('emptyCase', () => {
  it('records the workspace version from Cargo.toml as the app version', () => {
    const manifest = readFileSync(new URL('../../../Cargo.toml', import.meta.url), 'utf8');
    const version = /\[workspace\.package\]\s+version = "([^"]+)"/.exec(manifest)?.[1];
    expect(APP_VERSION).toBe(version);
  });

  it('holds no film, no marks, and gives out ids from 1', () => {
    expect(emptyCase()).toStrictEqual({
      format: 'spine-case',
      version: 1,
      savedWith: { app: APP_VERSION },
      films: [],
      marks: [],
      nextVertebraId: 1,
    });
  });

  it('is a new object each time, so no caller shares one', () => {
    expect(emptyCase()).not.toBe(emptyCase());
  });
});
