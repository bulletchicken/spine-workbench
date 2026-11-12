import { afterEach, describe, expect, it, vi } from 'vitest';
import { Button } from './Button';
import { takeKeys } from './keyboard/keyboard';
import { find, focusOn, pressKey, pressOn, putOnThePage } from './test-rig/on-the-page';

const letGo: (() => void)[] = [];
afterEach(() => {
  for (const stop of letGo.splice(0)) stop();
});

describe('Button', () => {
  it('runs its press', () => {
    const press = vi.fn();
    putOnThePage(<Button onPress={press}>Not yet</Button>);
    pressOn(find('button'));
    expect(press).toHaveBeenCalledTimes(1);
  });

  it('is a plain button, so a form around it is never submitted by it', () => {
    putOnThePage(<Button>Not yet</Button>);
    expect(find('button').getAttribute('type')).toBe('button');
  });

  it('when disabled takes no press and no focus', () => {
    const press = vi.fn();
    putOnThePage(
      <Button disabled onPress={press}>
        Save case
      </Button>,
    );
    const button = find('button');
    pressOn(button);
    focusOn(button);
    expect(press).not.toHaveBeenCalled();
    expect(document.activeElement).not.toBe(button);
  });

  it('when busy says so, takes no second press, and keeps the focus it had', () => {
    const press = vi.fn();
    const page = putOnThePage(<Button onPress={press}>Save case</Button>);
    const button = find('button');
    focusOn(button);
    pressOn(button);
    page.show(
      <Button busy onPress={press}>
        Save case
      </Button>,
    );
    expect(button.getAttribute('aria-busy')).toBe('true');
    expect(button.getAttribute('aria-disabled')).toBe('true');
    pressOn(button);
    expect(press).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(button);
    // Its name is still its words: they are drawn clear, not removed.
    expect(button.textContent).toBe('Save case');
  });

  it('keeps Enter to itself when focused: the lit button is not pressed as well', () => {
    const pressLit = vi.fn();
    letGo.push(takeKeys('lit', ['Enter'], pressLit));
    putOnThePage(<Button>Skip for now</Button>);
    focusOn(find('button'));
    pressKey('Enter');
    expect(pressLit).not.toHaveBeenCalled();
  });

  it('wears the design system’s class for its look, carries its size as a data attribute, and passes other attributes through', () => {
    const page = putOnThePage(
      <Button look="text" size="large" className="guide-action" aria-expanded data-autofocus>
        Watch again
      </Button>,
    );
    const button = find('button');
    expect([...button.classList]).toEqual(['button', 'quiet', 'guide-action']);
    expect(button.dataset.size).toBe('large');
    expect(button.getAttribute('aria-expanded')).toBe('true');
    expect(button.hasAttribute('data-autofocus')).toBe(true);
    page.show(<Button>Done</Button>);
    expect([...find('button').classList]).toEqual(['button', 'secondary']);
  });
});
