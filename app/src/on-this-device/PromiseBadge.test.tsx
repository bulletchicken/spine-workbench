// The promise in the top bar: the badge and its popover, wired to the
// page's own counter as the app wires them. A refusal by the content
// policy is raised here as the browser raises it, as an event on the
// document; the browser tests hold the policy itself.

import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { find, focusOn, pressKey, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import scenes from './on-this-device.scenes';
import { popoverOpen, usePopoverOpen } from './popover-open';
import { PromiseBadge } from './PromiseBadge';
import { useRequestsSent } from './requests-sent';

/** The badge as the frame mounts it: on the app's popover flag and the page's counter. */
function Wired() {
  const { open } = usePopoverOpen();
  return (
    <PromiseBadge
      requests={useRequestsSent()}
      open={open}
      onOpenChange={(next) => {
        popoverOpen.getState().setOpen(next);
      }}
    />
  );
}

/** The browser refusing `address` under the policy's `rule`, as it tells the page. */
function refuse(address: string, rule: string): void {
  const refusal = Object.assign(new Event('securitypolicyviolation'), { blockedURI: address, effectiveDirective: rule });
  act(() => {
    document.dispatchEvent(refusal);
  });
}

describe('the promise', () => {
  it('reads "0 sent" on a page that has asked nothing of anywhere else', () => {
    putOnThePage(<Wired />);
    expect(find('.promise-badge').textContent).toBe('On this device only0 sent');
    expect(find('.promise-badge').getAttribute('aria-expanded')).toBe('false');
    expect(document.querySelector('[role="dialog"]')).toBeNull();
  });

  it('opens from the badge, and Escape closes it and gives focus back', () => {
    putOnThePage(<Wired />);
    const badge = find('.promise-badge');
    focusOn(badge);
    // Enter or Space on the focused badge is its click.
    pressOn(badge);
    const popover = find('[role="dialog"]');
    expect(popover.getAttribute('aria-label')).toBe('On this device only');
    expect(badge.getAttribute('aria-expanded')).toBe('true');
    expect(popover.textContent).toContain('Nothing leaves this computer.');
    expect(find('.promise-count').textContent).toBe('0requests sent this session');
    expect(document.querySelectorAll('.promise-guards li')).toHaveLength(3);
    expect(document.activeElement).toBe(popover);

    expect(pressKey('Escape').defaultPrevented).toBe(true);
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.activeElement).toBe(badge);
  });

  it('shows a line when the browser refuses an address outside the app, and counts attempts, not reports', () => {
    putOnThePage(<Wired />);
    pressOn(find('.promise-badge'));
    expect(document.querySelector('.promise-blocked')).toBeNull();

    // An outside picture, refused. The policy is held twice, so the browser says so twice.
    refuse('https://example.org/picture.png', 'img-src');
    refuse('https://example.org/picture.png', 'img-src');
    expect(find('.promise-blocked').textContent).toBe('The browser blocked 1 attempt to reach outside the app. Nothing was sent.');

    // A refusal that is not about an address outside changes nothing.
    refuse('eval', 'script-src');
    expect(find('.promise-blocked').textContent).toContain('1 attempt');

    refuse('https://example.net/', 'connect-src');
    expect(find('.promise-blocked').textContent).toContain('2 attempts');
    // Refused is not sent: the count stays at nothing.
    expect(find('.promise-count b').textContent).toBe('0');
    expect(find('.promise-badge-sent').textContent).toBe('0 sent');
    act(() => {
      popoverOpen.getState().setOpen(false);
    });
  });

  it('is opened by its scene, as a press would', async () => {
    putOnThePage(<Wired />);
    const scene = scenes.find((each) => each.name === 'on-this-device');
    await act(async () => {
      await scene?.enter({ engine: {} as never });
    });
    expect(find('[role="dialog"]').textContent).toContain('Nothing leaves this computer.');
    act(() => {
      popoverOpen.getState().setOpen(false);
    });
  });
});
