// The promise in the top bar: the badge and its note, wired to the
// page's own counter as the app wires them. A refusal by the content
// policy is raised here as the browser raises it, as an event on the
// document; the browser tests hold the policy itself.

import { act, useRef } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { CONTENT_POLICY, CONTENT_POLICY_HEADER } from '../../build-plugins/content-policy';
import { Popover } from '@/building-blocks/Popover';
import { find, focusOn, pressKey, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { ON_THIS_DEVICE_WORDS } from '@/words/on-this-device';
import scenes from './on-this-device.scenes';
import { popoverOpen, usePopoverOpen } from './popover-open';
import { PromiseBadge } from './PromiseBadge';
import { PromisePopover } from './PromisePopover';
import { useRequestsSent } from './requests-sent';

/** The same sibling note and shared anchor as the bar, without a bench. */
function Wired() {
  const { open } = usePopoverOpen();
  const badge = useRef<HTMLButtonElement>(null);
  const requests = useRequestsSent();
  const setOpen = (next: boolean) => { popoverOpen.getState().setOpen(next); };
  return (
    <div className="frame">
      <PromiseBadge ref={badge} open={open} onOpenChange={setOpen} />
      <Popover anchor={badge} open={open} onClose={() => { setOpen(false); }} label={ON_THIS_DEVICE_WORDS.badge} className="note glass">
        <PromisePopover requests={requests} />
      </Popover>
    </div>
  );
}

/** The browser refusing `address` under the policy's `rule`, as it tells the page. */
function refuse(address: string, rule: string, policy = CONTENT_POLICY): void {
  const refusal = Object.assign(new Event('securitypolicyviolation'), { blockedURI: address, effectiveDirective: rule, originalPolicy: policy });
  act(() => {
    document.dispatchEvent(refusal);
  });
}

beforeEach(() => { popoverOpen.getState().setOpen(false); });

describe('the promise', () => {
  it('shows no count on the badge and keeps its closed note in the page', () => {
    putOnThePage(<Wired />);
    const badge = find('.badge');
    expect(badge.textContent).toBe('On this device only');
    expect(badge.getAttribute('aria-expanded')).toBe('false');
    expect(badge.getAttribute('aria-haspopup')).toBe('dialog');
    expect(find('.badge svg').getAttribute('viewBox')).toBe('0 0 18 18');
    expect(find('.badge rect').getAttribute('width')).toBe('10.5');
    expect(find('.badge path').getAttribute('d')).toBe('M6 8V5.75a3 3 0 0 1 6 0V8');
    expect(find('.note').classList.contains('open')).toBe(false);
    expect(find('.note').getAttribute('aria-hidden')).toBe('true');
    expect(find('.note').hasAttribute('inert')).toBe(true);
  });

  it('says the lead line, live request count and bone credit, and closes on Escape with focus returned', () => {
    putOnThePage(<Wired />);
    const badge = find('.badge');
    focusOn(badge);
    pressOn(badge);
    const note = find('[role="dialog"]');
    expect(note.getAttribute('aria-label')).toBe('On this device only');
    expect(badge.getAttribute('aria-expanded')).toBe('true');
    expect(find('.note-lead').textContent).toBe('Your films never leave this computer.');
    expect(find('.note-count').textContent).toBe('Requests sent: 0');
    expect(find('.note-count .num').textContent).toBe('0');
    expect(find('.note-credit').textContent).toBe(ON_THIS_DEVICE_WORDS.boneCredit);
    expect(document.activeElement).toBe(note);
    expect(pressKey('Escape').defaultPrevented).toBe(true);
    expect(find('[role="dialog"]')).toBe(note);
    expect(note.classList.contains('open')).toBe(false);
    expect(document.activeElement).toBe(badge);
  });

  it('closes on a second badge press, a press outside, and focus moving on', () => {
    putOnThePage(<><Wired /><button type="button" id="outside">Outside</button></>);
    const badge = find('.badge');
    pressOn(badge);
    pressOn(badge);
    expect(find('.note').classList.contains('open')).toBe(false);
    pressOn(badge);
    pressOn(find('#outside'));
    expect(find('.note').classList.contains('open')).toBe(false);
    pressOn(badge);
    focusOn(find('#outside'));
    expect(find('.note').classList.contains('open')).toBe(false);
    expect(document.activeElement).toBe(find('#outside'));
  });

  it('counts refused attempts rather than duplicate reports, under the request count', () => {
    putOnThePage(<Wired />);
    pressOn(find('.badge'));
    expect(document.querySelector('.note-blocked')).toBeNull();

    // The header and tag each report one refusal. Even a whole second apart, it is one attempt.
    vi.useFakeTimers({ toFake: ['Date'] });
    try {
      refuse('https://example.org/picture.png', 'img-src', CONTENT_POLICY);
      vi.setSystemTime(Date.now() + 1000);
      refuse('https://example.org/picture.png', 'img-src', CONTENT_POLICY_HEADER);
    } finally {
      vi.useRealTimers();
    }
    expect(find('.note-blocked').textContent).toBe('The browser blocked 1 attempt to reach outside the app. Nothing was sent.');
    expect(find('.note-blocked').previousElementSibling).toBe(find('.note-count'));

    // A refusal that is not about an address outside changes nothing.
    refuse('eval', 'script-src');
    expect(find('.note-blocked').textContent).toContain('1 attempt');

    refuse('https://example.net/', 'connect-src');
    expect(find('.note-blocked').textContent).toContain('2 attempts');
    // Refused is not sent: the count stays at nothing.
    expect(find('.note-count .num').textContent).toBe('0');
    expect(find('.badge').textContent).toBe('On this device only');
  });

  it('is opened by its scene, as a press would', async () => {
    putOnThePage(<Wired />);
    const scene = scenes.find((each) => each.name === 'on-this-device');
    await act(async () => { await scene?.enter({ engine: {} as never }); });
    expect(find('.note').classList.contains('open')).toBe(true);
    expect(find('.note-lead').textContent).toBe('Your films never leave this computer.');
  });
});
