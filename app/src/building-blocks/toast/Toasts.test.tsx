import { act } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { endAnimation, find, focusOn, pressOn, putOnThePage } from '../test-rig/on-the-page';
import { putAway, saidNow, say } from './say';
import { Toasts } from './Toasts';

// Stand-ins for the two icons the bench hands in.
const MARKS = { done: <i data-testid="tick" />, note: <i data-testid="info" /> };

const tell = (...args: Parameters<typeof say>) => {
  act(() => {
    say(...args);
  });
};
const toast = () => find('.toast');
const isShown = () => toast().hasAttribute('data-shown');
const spoken = () => find('[role="status"]').textContent;

afterEach(() => {
  const left = saidNow();
  if (left !== null) putAway(left.id);
});

describe('say() and Toasts', () => {
  it('shows nothing and says nothing until something is said', () => {
    putOnThePage(<Toasts marks={MARKS} />);
    expect(isShown()).toBe(false);
    expect(spoken()).toBe('');
  });

  it('shows one line, and speaks it as a status message', () => {
    putOnThePage(<Toasts marks={MARKS} />);
    tell('Split T12 in two. Levels renamed to match.');
    expect(isShown()).toBe(true);
    expect(find('.toast-line').textContent).toBe('Split T12 in two. Levels renamed to match.');
    expect(spoken()).toBe('Split T12 in two. Levels renamed to match.');
    // Spoken once: the line on the pill is for the eye only.
    expect(find('.toast-line').getAttribute('aria-hidden')).toBe('true');
  });

  it('has the status message on the page before anything is said, so the first line is heard', () => {
    putOnThePage(<Toasts marks={MARKS} />);
    const status = find('[role="status"]');
    tell('Case opened. Everything is as you left it.');
    expect(find('[role="status"]')).toBe(status);
  });

  it('is put away when it has stayed its time', () => {
    putOnThePage(<Toasts marks={MARKS} />);
    tell('Size set from your two clicks: 0.151 mm per pixel.');
    endAnimation(find('.toast-clock'));
    expect(isShown()).toBe(false);
    expect(spoken()).toBe('');
    expect(saidNow()).toBeNull();
    // The line is still drawn while the pill fades.
    expect(find('.toast-line').textContent).toBe('Size set from your two clicks: 0.151 mm per pixel.');
  });

  it('shows the newest line in place of the one on show, and starts its time afresh', () => {
    putOnThePage(<Toasts marks={MARKS} />);
    tell('Split T12 in two. Levels renamed to match.');
    const firstClock = find('.toast-clock');
    tell('Undid: split T12.');
    expect(find('.toast-line').textContent).toBe('Undid: split T12.');
    expect(spoken()).toBe('Undid: split T12.');
    expect(document.querySelectorAll('.toast')).toHaveLength(1);
    expect(find('.toast-clock')).not.toBe(firstClock);
  });

  it('speaks the same line again when it is said again', () => {
    putOnThePage(<Toasts marks={MARKS} />);
    tell('No disc gap found inside T8.', { tone: 'note' });
    const first = find('[role="status"] span');
    tell('No disc gap found inside T8.', { tone: 'note' });
    expect(find('[role="status"] span')).not.toBe(first);
    expect(spoken()).toBe('No disc gap found inside T8.');
  });

  it('offers its one action, with the keys that do the same, and is put away when the action is pressed', () => {
    const undo = vi.fn();
    putOnThePage(<Toasts marks={MARKS} />);
    tell('Split T12 in two. Levels renamed to match.', { action: { label: 'Undo', keys: ['Mod', 'z'], press: undo } });
    const action = find('.toast-action');
    expect(action.textContent).toMatch(/^Undo(⌘Z|Ctrl\+Z)$/);
    pressOn(action);
    expect(undo).toHaveBeenCalledTimes(1);
    expect(isShown()).toBe(false);
  });

  it('keeps a line the action itself says', () => {
    putOnThePage(<Toasts marks={MARKS} />);
    tell('Split T12 in two. Levels renamed to match.', {
      action: {
        label: 'Undo',
        press: () => {
          say('Undid: split T12.');
        },
      },
    });
    pressOn(find('.toast-action'));
    expect(isShown()).toBe(true);
    expect(find('.toast-line').textContent).toBe('Undid: split T12.');
    expect(document.querySelector('.toast-action')).toBeNull();
  });

  it('opens a line with the mark it was handed for the line’s tone: done unless it is said as a note', () => {
    putOnThePage(<Toasts marks={MARKS} />);
    tell('L5 set. The one below it is the sacrum. Levels renamed.');
    expect(find('.toast-mark').dataset.tone).toBe('done');
    expect(find('.toast-mark').querySelector('[data-testid="tick"]')).not.toBeNull();
    expect(find('.toast-mark').querySelector('[data-testid="info"]')).toBeNull();
    tell('Bending films carry endplate lines only.', { tone: 'note' });
    expect(find('.toast-mark').dataset.tone).toBe('note');
    expect(find('.toast-mark').querySelector('[data-testid="info"]')).not.toBeNull();
    expect(find('.toast-mark').querySelector('[data-testid="tick"]')).toBeNull();
    // For the eye only: the line is what is spoken.
    expect(find('.toast-mark').getAttribute('aria-hidden')).toBe('true');
  });

  it('an old line’s time running out does not put a newer line away', () => {
    putOnThePage(<Toasts marks={MARKS} />);
    tell('Split T12 in two. Levels renamed to match.');
    const old = saidNow();
    tell('Undid: split T12.');
    if (old !== null) putAway(old.id);
    expect(saidNow()?.line).toBe('Undid: split T12.');
  });

  describe('focus, when its action goes away while holding it', () => {
    /** A bench button with focus, the toasts after it, and a second button. */
    function bench() {
      putOnThePage(
        <>
          <button type="button" data-testid="split">
            Split T12
          </button>
          <Toasts marks={MARKS} />
          <button type="button" data-testid="other">
            Pins
          </button>
        </>,
      );
      const split = find('[data-testid="split"]');
      focusOn(split);
      return split;
    }

    it('goes back to where it came from when the action is pressed', () => {
      const split = bench();
      tell('Split T12 in two. Levels renamed to match.', { action: { label: 'Undo', press: () => undefined } });
      const action = find('.toast-action');
      focusOn(action);
      pressOn(action);
      // The pill is still drawn while it fades, with the button in it.
      expect(document.querySelector('.toast-action')).toBe(action);
      expect(document.activeElement).toBe(split);
    });

    it('goes back when the action says a line with no action, and its button leaves the page', () => {
      const split = bench();
      tell('Split T12 in two. Levels renamed to match.', {
        action: {
          label: 'Undo',
          press: () => {
            say('Undid: split T12.');
          },
        },
      });
      const action = find('.toast-action');
      focusOn(action);
      pressOn(action);
      expect(document.querySelector('.toast-action')).toBeNull();
      expect(document.activeElement).toBe(split);
    });

    it('goes back when a newer line with no action takes the toast’s place', () => {
      const split = bench();
      tell('Split T12 in two. Levels renamed to match.', { action: { label: 'Undo', press: () => undefined } });
      focusOn(find('.toast-action'));
      tell('Undid: split T12.');
      expect(document.activeElement).toBe(split);
    });

    it('stays on the action when a newer line has an action too', () => {
      bench();
      tell('Split T12 in two. Levels renamed to match.', { action: { label: 'Undo', press: () => undefined } });
      focusOn(find('.toast-action'));
      tell('Merged T11 and T12. Levels renamed to match.', { action: { label: 'Undo', press: () => undefined } });
      expect(document.activeElement).toBe(find('.toast-action'));
    });

    it('is left alone when it has already moved on from the toast', () => {
      bench();
      tell('Split T12 in two. Levels renamed to match.', { action: { label: 'Undo', press: () => undefined } });
      const action = find('.toast-action');
      focusOn(action);
      const other = find('[data-testid="other"]');
      focusOn(other);
      // A press that does not move focus to the button, as a pointer's does not in every browser.
      pressOn(action);
      expect(document.activeElement).toBe(other);
    });
  });
});
