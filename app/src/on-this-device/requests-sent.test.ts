// The counter tells a request by where its address leads. The page in
// these tests lives at https://spine.example: anything else is outside.

import { describe, expect, it } from 'vitest';
import { createTally, leadsOutside } from './requests-sent';

const OWN = 'https://spine.example';
const TAG_POLICY = "default-src 'self'";
const HEADER_POLICY = `${TAG_POLICY}; frame-ancestors 'self'`;

// Every order of the four reports for two attempts, without relying on browser scheduling.
const REPORT_ORDERS = [
  ['a', 'b', 'c', 'd'],
  ['a', 'b', 'd', 'c'],
  ['a', 'c', 'b', 'd'],
  ['a', 'c', 'd', 'b'],
  ['a', 'd', 'b', 'c'],
  ['a', 'd', 'c', 'b'],
  ['b', 'a', 'c', 'd'],
  ['b', 'a', 'd', 'c'],
  ['b', 'c', 'a', 'd'],
  ['b', 'c', 'd', 'a'],
  ['b', 'd', 'a', 'c'],
  ['b', 'd', 'c', 'a'],
  ['c', 'a', 'b', 'd'],
  ['c', 'a', 'd', 'b'],
  ['c', 'b', 'a', 'd'],
  ['c', 'b', 'd', 'a'],
  ['c', 'd', 'a', 'b'],
  ['c', 'd', 'b', 'a'],
  ['d', 'a', 'b', 'c'],
  ['d', 'a', 'c', 'b'],
  ['d', 'b', 'a', 'c'],
  ['d', 'b', 'c', 'a'],
  ['d', 'c', 'a', 'b'],
  ['d', 'c', 'b', 'a'],
] as const;

describe('an address', () => {
  it('leads outside when its origin is not the page’s own', () => {
    expect(leadsOutside('https://example.org/picture.png', OWN)).toBe(true);
    expect(leadsOutside('https://example.org', OWN)).toBe(true);
    expect(leadsOutside('wss://example.org/socket', OWN)).toBe(true);
    // Another port, and plain http, are other origins too.
    expect(leadsOutside('https://spine.example:8443/app.js', OWN)).toBe(true);
    expect(leadsOutside('http://spine.example/app.js', OWN)).toBe(true);
  });

  it('stays inside when it is the page’s own origin', () => {
    expect(leadsOutside('https://spine.example/assets/engine.wasm', OWN)).toBe(false);
    expect(leadsOutside('blob:https://spine.example/0d5c3a0e-film', OWN)).toBe(false);
  });

  it('leads nowhere when it is not an address at all', () => {
    for (const word of ['eval', 'inline', 'wasm-eval', 'blob', 'data', '']) expect(leadsOutside(word, OWN), word).toBe(false);
    expect(leadsOutside('data:image/png;base64,AAAA', OWN)).toBe(false);
  });
});

describe('the tally', () => {
  it('starts at nothing sent and nothing blocked', () => {
    expect(createTally(OWN).read()).toEqual({ sent: 0, blocked: 0 });
  });

  it('counts what was asked of another origin, and not the app’s own files', () => {
    const tally = createTally(OWN);
    tally.asked('https://spine.example/assets/index.js');
    tally.asked('https://spine.example/assets/engine.wasm');
    expect(tally.read().sent).toBe(0);
    tally.asked('https://example.org/track.gif');
    tally.asked('https://fonts.example/geist.woff2');
    expect(tally.read()).toEqual({ sent: 2, blocked: 0 });
  });

  it('does not count as sent what the policy refused, whichever the browser tells of first', () => {
    // The browser lists a refused request in its record as well as raising the refusal.
    const refusalFirst = createTally(OWN);
    refusalFirst.refused({ address: 'https://example.org/picture.png', rule: 'img-src', policy: TAG_POLICY });
    refusalFirst.asked('https://example.org/picture.png');
    expect(refusalFirst.read()).toEqual({ sent: 0, blocked: 1 });

    const recordFirst = createTally(OWN);
    recordFirst.asked('https://example.org/picture.png#top');
    expect(recordFirst.read().sent).toBe(1);
    recordFirst.refused({ address: 'https://example.org/picture.png', rule: 'img-src', policy: TAG_POLICY });
    expect(recordFirst.read()).toEqual({ sent: 0, blocked: 1 });
  });

  it('takes a refusal that names only an origin to cover every address there, and no other origin', () => {
    const tally = createTally(OWN);
    tally.asked('https://example.org/a/deep/path?x=1');
    tally.asked('https://example.net/other');
    tally.refused({ address: 'https://example.org', rule: 'connect-src', policy: TAG_POLICY });
    expect(tally.read()).toEqual({ sent: 1, blocked: 1 });
  });

  it('counts one refusal under two distinct policies as one attempt', () => {
    const tally = createTally(OWN);
    tally.refused({ address: 'https://example.org/picture.png', rule: 'img-src', policy: TAG_POLICY });
    tally.refused({ address: 'https://example.org/picture.png', rule: 'img-src', policy: HEADER_POLICY });
    expect(tally.read()).toEqual({ sent: 0, blocked: 1 });
  });

  it.each(REPORT_ORDERS)('counts two same-address attempts under two policies in order %s %s %s %s', (first, second, third, fourth) => {
    const tally = createTally(OWN);
    const reports = {
      a: { address: 'https://example.org/', rule: 'connect-src', policy: TAG_POLICY },
      b: { address: 'https://example.org/', rule: 'connect-src', policy: HEADER_POLICY },
      c: { address: 'https://example.org/', rule: 'connect-src', policy: TAG_POLICY },
      d: { address: 'https://example.org/', rule: 'connect-src', policy: HEADER_POLICY },
    };
    // No clock input: these orders cover close attempts and distant ones alike.
    for (const report of [first, second, third, fourth]) tally.refused(reports[report]);
    expect(tally.read()).toEqual({ sent: 0, blocked: 2 });
  });

  it('counts two consecutive refusals of the same address when the policy is held once', () => {
    const tally = createTally(OWN);
    tally.refused({ address: 'https://example.org/', rule: 'connect-src', policy: TAG_POLICY });
    expect(tally.read().blocked).toBe(1);
    tally.refused({ address: 'https://example.org/', rule: 'connect-src', policy: TAG_POLICY });
    expect(tally.read().blocked).toBe(2);
  });

  it.each(REPORT_ORDERS)('counts two different addresses under two policies in order %s %s %s %s', (first, second, third, fourth) => {
    const tally = createTally(OWN);
    const reports = {
      a: { address: 'https://example.org/a', rule: 'img-src', policy: TAG_POLICY },
      b: { address: 'https://example.org/a', rule: 'img-src', policy: HEADER_POLICY },
      c: { address: 'https://example.net/b', rule: 'img-src', policy: TAG_POLICY },
      d: { address: 'https://example.net/b', rule: 'img-src', policy: HEADER_POLICY },
    };
    for (const report of [first, second, third, fourth]) tally.refused(reports[report]);
    expect(tally.read()).toEqual({ sent: 0, blocked: 2 });
  });

  it('keeps an upper bound when a host repeats the tag policy text in its header', () => {
    const tally = createTally(OWN);
    tally.refused({ address: 'https://example.org/picture.png', rule: 'img-src', policy: TAG_POLICY });
    tally.refused({ address: 'https://example.org/picture.png', rule: 'img-src', policy: TAG_POLICY });
    expect(tally.read()).toEqual({ sent: 0, blocked: 2 });
  });

  it('counts different addresses, and different rules, apart', () => {
    const tally = createTally(OWN);
    tally.refused({ address: 'https://example.org/', rule: 'connect-src', policy: TAG_POLICY });
    tally.refused({ address: 'https://example.net/', rule: 'connect-src', policy: TAG_POLICY });
    tally.refused({ address: 'https://example.org/', rule: 'img-src', policy: TAG_POLICY });
    expect(tally.read().blocked).toBe(3);
  });

  it('does not count a refusal that is not about an address outside: eval, or the app’s own origin', () => {
    const tally = createTally(OWN);
    tally.refused({ address: 'eval', rule: 'script-src', policy: TAG_POLICY });
    tally.refused({ address: 'inline', rule: 'style-src-elem', policy: TAG_POLICY });
    tally.refused({ address: 'blob', rule: 'worker-src', policy: TAG_POLICY });
    tally.refused({ address: 'https://spine.example/x', rule: 'frame-src', policy: TAG_POLICY });
    expect(tally.read()).toEqual({ sent: 0, blocked: 0 });
  });

  it('tells its watchers of each change, until they stop watching', () => {
    const tally = createTally(OWN);
    let told = 0;
    const stop = tally.watch(() => {
      told += 1;
    });
    tally.asked('https://example.org/a');
    tally.asked('https://spine.example/own');
    tally.refused({ address: 'https://example.org/b', rule: 'img-src', policy: TAG_POLICY });
    // The other policy reporting the same attempt leaves the count unchanged.
    tally.refused({ address: 'https://example.org/b', rule: 'img-src', policy: HEADER_POLICY });
    expect(told).toBe(2);
    stop();
    tally.asked('https://example.org/c');
    expect(told).toBe(2);
  });
});
