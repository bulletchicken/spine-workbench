import { useState, type ReactNode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Dialog } from './Dialog';
import { takeKeys } from './keyboard/keyboard';
import { endAnimation, find, focusOn, pressKey, pressOn, putOnThePage, send } from './test-rig/on-the-page';

/** A guide button outside the bench, and a bench with a dialog in it. */
function Room({ children, onClose = () => undefined }: { children: ReactNode; onClose?: () => void }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button
        type="button"
        data-testid="opener"
        onClick={() => {
          setOpen(true);
        }}
      >
        Save the case
      </button>
      <div data-testid="bench">
        <button type="button" data-testid="under-the-veil">
          Pins
        </button>
        <Dialog
          open={open}
          onClose={() => {
            onClose();
            setOpen(false);
          }}
          label="Save the case"
        >
          {children}
        </Dialog>
      </div>
    </>
  );
}

const TWO_BUTTONS = (
  <>
    <button type="button" data-testid="save">
      Save case file
    </button>
    <button type="button" data-testid="not-yet">
      Not yet
    </button>
  </>
);

const dialog = () => document.querySelector<HTMLElement>('[role="dialog"]');

const letGo: (() => void)[] = [];
afterEach(() => {
  for (const stop of letGo.splice(0)) stop();
});

function open(children: ReactNode, onClose?: () => void) {
  putOnThePage(onClose === undefined ? <Room>{children}</Room> : <Room onClose={onClose}>{children}</Room>);
  const opener = find('[data-testid="opener"]');
  focusOn(opener);
  pressOn(opener);
  return opener;
}

describe('Dialog', () => {
  it('is not on the page until it is open; then it is a named, modal dialog', () => {
    putOnThePage(<Room>{TWO_BUTTONS}</Room>);
    expect(dialog()).toBeNull();
    pressOn(find('[data-testid="opener"]'));
    expect(dialog()?.getAttribute('aria-modal')).toBe('true');
    expect(dialog()?.getAttribute('aria-label')).toBe('Save the case');
  });

  it('veils the region it is mounted in, not the page', () => {
    open(TWO_BUTTONS);
    const veil = find('.dialog-veil');
    expect(veil.parentElement).toBe(find('[data-testid="bench"]'));
  });

  it('puts focus on its first control', () => {
    open(TWO_BUTTONS);
    expect(document.activeElement).toBe(find('[data-testid="save"]'));
  });

  it('puts focus on the control marked data-autofocus instead, when there is one', () => {
    open(
      <>
        <button type="button">Save case file</button>
        <button type="button" data-autofocus data-testid="not-yet">
          Not yet
        </button>
      </>,
    );
    expect(document.activeElement).toBe(find('[data-testid="not-yet"]'));
  });

  it('puts focus on the sheet itself when nothing inside can take it', () => {
    open(<p>Saved to this computer.</p>);
    expect(document.activeElement).toBe(dialog());
  });

  it('sends Tab round: from the last control to the first, and Shift+Tab back again', () => {
    open(TWO_BUTTONS);
    const save = find('[data-testid="save"]');
    const notYet = find('[data-testid="not-yet"]');

    focusOn(notYet);
    expect(pressKey('Tab').defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(save);

    expect(pressKey('Tab', { shiftKey: true }).defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(notYet);
  });

  it('leaves Tab between its controls to the browser', () => {
    open(TWO_BUTTONS);
    expect(pressKey('Tab').defaultPrevented).toBe(false);
  });

  it('brings back focus that lands outside it, to where it last was inside', () => {
    open(TWO_BUTTONS);
    const notYet = find('[data-testid="not-yet"]');
    focusOn(notYet);
    focusOn(find('[data-testid="under-the-veil"]'));
    expect(document.activeElement).toBe(notYet);
    focusOn(find('[data-testid="opener"]'));
    expect(document.activeElement).toBe(notYet);
  });

  it('lets focus go into another dialog or popover opened over it', () => {
    open(TWO_BUTTONS);
    const popover = document.createElement('div');
    popover.setAttribute('role', 'dialog');
    popover.tabIndex = -1;
    document.body.append(popover);
    focusOn(popover);
    expect(document.activeElement).toBe(popover);
    popover.remove();
  });

  it('closes on Escape; focus returns to where it was; the sheet leaves once it has faded', () => {
    const closed = vi.fn();
    const opener = open(TWO_BUTTONS, closed);
    const event = pressKey('Escape');
    expect(closed).toHaveBeenCalledTimes(1);
    expect(event.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(opener);

    // Still drawn while it fades, but no longer something to use.
    const leaving = dialog();
    expect(leaving?.hasAttribute('data-leaving')).toBe(true);
    expect(leaving?.hasAttribute('inert')).toBe(true);
    expect(find('.dialog-veil').hasAttribute('data-leaving')).toBe(true);

    endAnimation(find('[role="dialog"]'));
    expect(dialog()).toBeNull();
    expect(document.querySelector('.dialog-veil')).toBeNull();
  });

  it('gives focus back once: where the person goes while it fades is left alone', () => {
    open(TWO_BUTTONS);
    pressKey('Escape');
    const elsewhere = find('[data-testid="under-the-veil"]');
    focusOn(elsewhere);
    endAnimation(find('[role="dialog"]'));
    expect(dialog()).toBeNull();
    expect(document.activeElement).toBe(elsewhere);
  });

  it('gives focus back when it is taken off the page while still open', () => {
    function Shell({ withDialog }: { withDialog: boolean }) {
      return (
        <>
          <button type="button" data-testid="opener">
            Save the case
          </button>
          {withDialog && (
            <Dialog open onClose={() => undefined} label="Save the case">
              {TWO_BUTTONS}
            </Dialog>
          )}
        </>
      );
    }
    const page = putOnThePage(<Shell withDialog={false} />);
    const opener = find('[data-testid="opener"]');
    focusOn(opener);
    page.show(<Shell withDialog />);
    expect(document.activeElement).toBe(find('[data-testid="save"]'));
    page.show(<Shell withDialog={false} />);
    expect(dialog()).toBeNull();
    expect(document.activeElement).toBe(opener);
  });

  it('swallows a press on the veil, so focus stays in the sheet', () => {
    open(TWO_BUTTONS);
    const down = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
    send(find('.dialog-veil'), down);
    expect(down.defaultPrevented).toBe(true);
  });

  it('does not answer Escape a second time while it fades', () => {
    const closed = vi.fn();
    open(TWO_BUTTONS, closed);
    pressKey('Escape');
    pressKey('Escape');
    expect(closed).toHaveBeenCalledTimes(1);
  });

  it('does not leave because an animation inside it ended', () => {
    open(TWO_BUTTONS);
    pressKey('Escape');
    endAnimation(find('[data-testid="save"]'));
    expect(dialog()).not.toBeNull();
  });

  it('takes Escape before a popover and before the review', () => {
    const closePopover = vi.fn();
    const leaveReview = vi.fn();
    letGo.push(takeKeys('popover', ['Escape'], closePopover), takeKeys('review', ['Escape'], leaveReview));
    open(TWO_BUTTONS);
    pressKey('Escape');
    expect(closePopover).not.toHaveBeenCalled();
    expect(leaveReview).not.toHaveBeenCalled();
  });

  describe('and the keys of the parts behind it', () => {
    /** What the bench and the guide answer while a sheet can be open over them, and undo. */
    function partsBehind() {
      const behind = vi.fn();
      const undo = vi.fn();
      letGo.push(
        takeKeys('choice', ['a', 'b', 'ArrowLeft', 'ArrowRight'], behind),
        takeKeys('lit', ['Enter'], behind),
        takeKeys('pass', ['Space'], behind),
        takeKeys('tools', ['v', 'r', 'b', 'm'], behind),
        takeKeys('cameras', ['0', '+', '-'], behind),
        takeKeys('always', ['z'], undo),
      );
      return { behind, undo };
    }

    it('keeps every key from them while it is open; undo is still answered', () => {
      const { behind, undo } = partsBehind();
      // The takers are live: before the sheet opens, B is answered.
      pressKey('b');
      expect(behind).toHaveBeenCalledTimes(1);
      behind.mockClear();

      open(TWO_BUTTONS);
      focusOn(find('[role="dialog"]'));
      for (const key of ['b', 'ArrowLeft', '0', ' ', 'Enter']) {
        expect(pressKey(key).defaultPrevented, key).toBe(false);
      }
      // From a button in the sheet as well, where Enter and Space are the button's own.
      focusOn(find('[data-testid="save"]'));
      for (const key of ['a', 'b', 'ArrowLeft', 'v']) pressKey(key);
      expect(behind).not.toHaveBeenCalled();

      pressKey('z', { metaKey: true });
      expect(undo).toHaveBeenCalledTimes(1);
    });

    it('gives them their keys back as soon as it starts to leave', () => {
      const { behind } = partsBehind();
      open(TWO_BUTTONS);
      pressKey('Escape');
      expect(dialog()?.hasAttribute('data-leaving')).toBe(true);
      pressKey('b');
      expect(behind).toHaveBeenCalledTimes(1);
    });

    it('gives them back when it is taken off the page while still open', () => {
      const { behind } = partsBehind();
      const page = putOnThePage(
        <Dialog open onClose={() => undefined} label="Save the case">
          {TWO_BUTTONS}
        </Dialog>,
      );
      pressKey('b');
      expect(behind).not.toHaveBeenCalled();
      page.takeDown();
      pressKey('b');
      expect(behind).toHaveBeenCalledTimes(1);
    });

    it('keeps them again when it opens again while still fading', () => {
      const { behind } = partsBehind();
      const opener = open(TWO_BUTTONS);
      pressKey('Escape');
      pressOn(opener);
      pressKey('b');
      expect(behind).not.toHaveBeenCalled();
    });
  });

  it('opens again if asked while it is still fading', () => {
    const opener = open(TWO_BUTTONS);
    pressKey('Escape');
    pressOn(opener);
    expect(dialog()?.hasAttribute('data-leaving')).toBe(false);
    expect(document.activeElement).toBe(find('[data-testid="save"]'));
  });
});
