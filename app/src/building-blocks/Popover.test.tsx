import { useRef, useState } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { takeKeys } from './keyboard/keyboard';
import { Popover } from './Popover';
import { find, focusOn, pressKey, pressOn, putOnThePage } from './test-rig/on-the-page';

/** A badge that opens a popover, with a second button beside it. */
function Badge({ onClose = () => undefined }: { onClose?: () => void }) {
  const [open, setOpen] = useState(false);
  const badge = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button
        type="button"
        ref={badge}
        data-testid="badge"
        onClick={() => {
          setOpen(!open);
        }}
      >
        On this device only
      </button>
      <Popover
        open={open}
        onClose={() => {
          onClose();
          setOpen(false);
        }}
        anchor={badge}
        label="On this device only"
        className="note glass"
      >
        <p>Nothing leaves this computer.</p>
        <a href="#guards">The three guards</a>
      </Popover>
      <button type="button" data-testid="elsewhere">
        Save case
      </button>
    </>
  );
}

/** The panel: always on the page, open or not. */
const popover = () => find('[role="dialog"]');
const isOpen = () => popover().classList.contains('open');

const letGo: (() => void)[] = [];
afterEach(() => {
  for (const stop of letGo.splice(0)) stop();
});

describe('Popover', () => {
  it('is on the page while closed, hidden from assistive technology and inert, without the class open', () => {
    putOnThePage(<Badge />);
    expect(popover().getAttribute('aria-label')).toBe('On this device only');
    expect(popover().getAttribute('aria-hidden')).toBe('true');
    expect(popover().hasAttribute('inert')).toBe(true);
    expect(isOpen()).toBe(false);
  });

  it('opens by adding the class open and nothing else of its own', () => {
    putOnThePage(<Badge />);
    pressOn(find('[data-testid="badge"]'));
    expect(popover().getAttribute('class')).toBe('note glass open');
    expect(popover().hasAttribute('aria-hidden')).toBe(false);
    expect(popover().hasAttribute('inert')).toBe(false);
  });

  it('takes focus when it opens, so its name is spoken', () => {
    putOnThePage(<Badge />);
    const badge = find('[data-testid="badge"]');
    focusOn(badge);
    pressOn(badge);
    expect(document.activeElement).toBe(popover());
  });

  it('closes on Escape, and focus returns to the control that opened it', () => {
    const closed = vi.fn();
    putOnThePage(<Badge onClose={closed} />);
    const badge = find('[data-testid="badge"]');
    focusOn(badge);
    pressOn(badge);
    const event = pressKey('Escape');
    expect(closed).toHaveBeenCalledTimes(1);
    expect(event.defaultPrevented).toBe(true);
    expect(isOpen()).toBe(false);
    expect(document.activeElement).toBe(badge);
  });

  it('takes Escape before the review does', () => {
    const leaveReview = vi.fn();
    letGo.push(takeKeys('review', ['Escape'], leaveReview));
    putOnThePage(<Badge />);
    pressOn(find('[data-testid="badge"]'));
    pressKey('Escape');
    expect(leaveReview).not.toHaveBeenCalled();
    pressKey('Escape');
    expect(leaveReview).toHaveBeenCalledTimes(1);
  });

  it('closes on a press outside it', () => {
    putOnThePage(<Badge />);
    pressOn(find('[data-testid="badge"]'));
    pressOn(document.body);
    expect(isOpen()).toBe(false);
  });

  it('stays open through a press inside it', () => {
    putOnThePage(<Badge />);
    pressOn(find('[data-testid="badge"]'));
    pressOn(find('p'));
    expect(isOpen()).toBe(true);
  });

  it('leaves a press on its own control to that control: one press closes it, and it does not open again', () => {
    const closed = vi.fn();
    putOnThePage(<Badge onClose={closed} />);
    const badge = find('[data-testid="badge"]');
    pressOn(badge);
    pressOn(badge);
    expect(closed).not.toHaveBeenCalled();
    expect(isOpen()).toBe(false);
  });

  it('closes when focus moves on to something else, and leaves focus there', () => {
    putOnThePage(<Badge />);
    pressOn(find('[data-testid="badge"]'));
    const elsewhere = find('[data-testid="elsewhere"]');
    focusOn(elsewhere);
    expect(isOpen()).toBe(false);
    expect(document.activeElement).toBe(elsewhere);
  });

  it('stays open while focus moves among the things inside it', () => {
    putOnThePage(<Badge />);
    pressOn(find('[data-testid="badge"]'));
    focusOn(find('a'));
    expect(isOpen()).toBe(true);
  });

  it('while closed takes no key: Escape is left to whoever comes next', () => {
    const closed = vi.fn();
    const leaveReview = vi.fn();
    letGo.push(takeKeys('review', ['Escape'], leaveReview));
    putOnThePage(<Badge onClose={closed} />);
    pressKey('Escape');
    expect(closed).not.toHaveBeenCalled();
    expect(leaveReview).toHaveBeenCalledTimes(1);
  });
});
