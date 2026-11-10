import { describe, expect, it } from 'vitest';
import { CONTENT_POLICY, POLICY_META, policyComesFirst } from './content-policy';

function directives(): Map<string, string[]> {
  return new Map(
    CONTENT_POLICY.split(';').map((directive) => {
      const [name = '', ...sources] = directive.trim().split(/\s+/);
      return [name, sources];
    }),
  );
}

describe('the content policy', () => {
  it('allows only this origin, and WebAssembly without eval', () => {
    const policy = directives();
    expect(policy.get('default-src')).toEqual(["'self'"]);
    expect(policy.get('script-src')).toEqual(["'self'", "'wasm-unsafe-eval'"]);
    expect(policy.get('connect-src')).toEqual(["'self'"]);
    expect(policy.get('worker-src')).toEqual(["'self'"]);
    expect(policy.get('object-src')).toEqual(["'none'"]);
    expect(policy.get('frame-src')).toEqual(["'none'"]);
    expect(policy.get('base-uri')).toEqual(["'none'"]);
    expect(policy.get('form-action')).toEqual(["'none'"]);
  });

  it('never allows inline code, eval or another host', () => {
    expect(CONTENT_POLICY).not.toMatch(/'unsafe-inline'|'unsafe-eval'|https?:|\*/);
  });

  it('must be the first element in the head, exactly', () => {
    expect(policyComesFirst(`<html><head>\n  ${POLICY_META}<meta charset="utf-8"></head></html>`)).toBe(true);
    expect(policyComesFirst('<html><head><meta charset="utf-8"></head></html>')).toBe(false);
    expect(policyComesFirst(`<html><head><meta charset="utf-8">${POLICY_META}</head></html>`)).toBe(false);
    const loosened = POLICY_META.replace("'wasm-unsafe-eval'", "'unsafe-eval'");
    expect(policyComesFirst(`<html><head>${loosened}</head></html>`)).toBe(false);
  });
});
