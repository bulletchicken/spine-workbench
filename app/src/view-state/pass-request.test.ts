import { describe, expect, it } from 'vitest';
import { createPassRequest } from './pass-request';

describe('pass request', () => {
  it('has nothing to take at the start', () => {
    const request = createPassRequest();
    expect(request.getState().requested).toBe(false);
    expect(request.getState().take()).toBe(false);
  });

  it('is taken once, and not again until the next request', () => {
    const request = createPassRequest();
    request.getState().request();
    expect(request.getState().take()).toBe(true);
    expect(request.getState().take()).toBe(false);
    request.getState().request();
    expect(request.getState().take()).toBe(true);
  });
});
