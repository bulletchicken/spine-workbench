// The four cases, written out: motion is reduced when either asks.

import { describe, expect, it } from 'vitest';
import { motionMode } from './reduced-motion';

describe('motionMode', () => {
  it('is full only when neither the system nor the app asks for less', () => {
    expect(motionMode(false, false)).toBe('full');
  });

  it('is reduced when the system asks', () => {
    expect(motionMode(true, false)).toBe('reduced');
  });

  it('is reduced when the app asks', () => {
    expect(motionMode(false, true)).toBe('reduced');
  });

  it('is reduced when both ask', () => {
    expect(motionMode(true, true)).toBe('reduced');
  });
});
