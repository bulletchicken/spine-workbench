import { afterEach, describe, expect, it, vi } from 'vitest';
import { IconButton } from './IconButton';
import { takeKeys } from './keyboard/keyboard';
import { find, focusOn, pointerOff, pointerOnto, pressKey, pressOn, putOnThePage, send, tabTo } from './test-rig/on-the-page';
import { Tooltip } from './Tooltip';

const isShown = () => find('[role="tooltip"]').hasAttribute('data-shown');

const letGo: (() => void)[] = [];
afterEach(() => {
  for (const stop of letGo.splice(0)) stop();
});

describe('Tooltip', () => {
  function onAButton() {
    putOnThePage(
      <Tooltip tip="Undo: Split T12">
        <button type="button">Undo</button>
      </Tooltip>,
    );
    return find('button');
  }

  it('is put away until the pointer rests on its control, and again when the pointer leaves', () => {
    const button = onAButton();
    expect(isShown()).toBe(false);
    pointerOnto(button);
    expect(isShown()).toBe(true);
    pointerOff(button);
    expect(isShown()).toBe(false);
  });

  it('shows while the keyboard’s focus is on its control', () => {
    const button = onAButton();
    tabTo(button);
    expect(isShown()).toBe(true);
    send(button, new FocusEvent('focusout', { bubbles: true }));
    expect(isShown()).toBe(false);
  });

  it('stays put away when a press, not the keyboard, gave its control focus', () => {
    const button = onAButton();
    pointerOnto(button);
    pressOn(button);
    focusOn(button);
    expect(isShown()).toBe(false);
  });

  it('is put away by a press on its control', () => {
    const button = onAButton();
    pointerOnto(button);
    pressOn(button);
    expect(isShown()).toBe(false);
  });

  it('is put away by Escape, which is still passed on to whatever else answers it', () => {
    const leaveReview = vi.fn();
    letGo.push(takeKeys('review', ['Escape'], leaveReview));
    const button = onAButton();
    pointerOnto(button);
    pressKey('Escape', {}, document.body);
    expect(isShown()).toBe(false);
    expect(leaveReview).toHaveBeenCalledTimes(1);
  });

  it('describes its control to a screen reader', () => {
    const button = onAButton();
    const tooltip = find('[role="tooltip"]');
    expect(tooltip.textContent).toBe('Undo: Split T12');
    expect(button.getAttribute('aria-describedby')).toBe(tooltip.id);
  });

  it('carries its look, side and place to CSS as data attributes', () => {
    putOnThePage(
      <Tooltip tip="Help" look="help" side="right" align="start">
        <button type="button">Pins</button>
      </Tooltip>,
    );
    expect(find('[role="tooltip"]').dataset).toMatchObject({ look: 'help', side: 'right', align: 'start' });
  });
});

describe('IconButton', () => {
  it('has a spoken name, and its tooltip names the action a press would do', () => {
    const undo = vi.fn();
    putOnThePage(
      <IconButton label="Undo" tip="Undo: Split T12" onPress={undo}>
        <svg />
      </IconButton>,
    );
    const button = find('button');
    const tooltip = find('[role="tooltip"]');
    expect(button.getAttribute('aria-label')).toBe('Undo');
    expect(tooltip.textContent).toBe('Undo: Split T12');
    expect(button.getAttribute('aria-describedby')).toBe(tooltip.id);
    pressOn(button);
    expect(undo).toHaveBeenCalledTimes(1);
  });

  it('with no more to say shows its name as the tooltip, and does not speak it twice', () => {
    putOnThePage(
      <IconButton label="Pause">
        <svg />
      </IconButton>,
    );
    const tooltip = find('[role="tooltip"]');
    expect(tooltip.textContent).toBe('Pause');
    expect(tooltip.getAttribute('aria-hidden')).toBe('true');
    expect(find('button').hasAttribute('aria-describedby')).toBe(false);
  });

  it('when disabled takes no press, yet can still be reached so its tooltip can say why', () => {
    const undo = vi.fn();
    putOnThePage(
      <IconButton label="Undo" tip="Nothing to undo" disabled onPress={undo}>
        <svg />
      </IconButton>,
    );
    const button = find('button');
    expect(button.getAttribute('aria-disabled')).toBe('true');
    tabTo(button);
    expect(document.activeElement).toBe(button);
    expect(isShown()).toBe(true);
    expect(find('[role="tooltip"]').textContent).toBe('Nothing to undo');
    pressOn(button);
    pressKey('Enter');
    expect(undo).not.toHaveBeenCalled();
  });
});
