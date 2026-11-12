import { afterEach, describe, expect, it, vi } from 'vitest';
import { takeKeys } from './keyboard/keyboard';
import { Segmented } from './Segmented';
import { find, focusOn, pressKey, pressOn, putOnThePage } from './test-rig/on-the-page';

const STEPS = [
  { id: 'films', label: 'Films' },
  { id: 'find', label: 'Find' },
  { id: 'later', label: 'Later', disabled: true },
] as const;

const letGo: (() => void)[] = [];
afterEach(() => {
  for (const stop of letGo.splice(0)) stop();
});

function option(name: string): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>('.seg button')].find((button) => button.textContent === name);
  if (found === undefined) throw new Error(`no option named ${name}`);
  return found;
}

describe('Segmented', () => {
  it('is a named list of places to go, and the chosen option is the current one', () => {
    putOnThePage(<Segmented label="Steps" options={STEPS} chosen="find" onChoose={() => undefined} />);
    expect(find('nav.seg').getAttribute('aria-label')).toBe('Steps');
    expect(option('Find').getAttribute('aria-current')).toBe('true');
    expect(option('Films').hasAttribute('aria-current')).toBe(false);
    expect(find('nav.seg').hasAttribute('data-none')).toBe(false);
  });

  it('wears the design system’s class, and the name of the control it is', () => {
    putOnThePage(<Segmented label="Steps" options={STEPS} chosen="films" onChoose={() => undefined} className="steps" />);
    expect(find('nav').getAttribute('class')).toBe('seg steps');
  });

  it('says when no option is chosen, so the thumb is hidden', () => {
    putOnThePage(<Segmented label="Steps" options={STEPS} chosen={null} onChoose={() => undefined} />);
    expect(find('nav.seg').hasAttribute('data-none')).toBe(true);
    expect(document.querySelector('[aria-current]')).toBeNull();
  });

  it('takes no press on a disabled option, and leaves it out of the tab order', () => {
    const choose = vi.fn();
    putOnThePage(<Segmented label="Steps" options={STEPS} chosen="films" onChoose={choose} />);
    const later = option('Later');
    pressOn(later);
    focusOn(later);
    expect(choose).not.toHaveBeenCalled();
    expect(later.disabled).toBe(true);
    expect(document.activeElement).not.toBe(later);
  });

  it('calls onChoose once, with the option’s id, for a press', () => {
    const choose = vi.fn();
    putOnThePage(<Segmented label="Steps" options={STEPS} chosen="films" onChoose={choose} />);
    pressOn(option('Find'));
    expect(choose).toHaveBeenCalledTimes(1);
    expect(choose).toHaveBeenCalledWith('find');
  });

  it('keeps Enter for the focused option: the browser presses it, and the lit button is not pressed', () => {
    const pressLit = vi.fn();
    letGo.push(takeKeys('lit', ['Enter'], pressLit));
    putOnThePage(<Segmented label="Steps" options={STEPS} chosen="films" onChoose={() => undefined} />);
    focusOn(option('Find'));
    // The key is left to the browser, whose press of a focused button is the click.
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    expect(pressLit).not.toHaveBeenCalled();
  });

  it('writes where the thumb sits as the chosen option’s place', () => {
    const page = putOnThePage(<Segmented label="Steps" options={STEPS} chosen="films" onChoose={() => undefined} />);
    const nav = find('nav.seg');
    expect(nav.style.getPropertyValue('--at')).toBe('0');
    page.show(<Segmented label="Steps" options={STEPS} chosen="find" onChoose={() => undefined} />);
    expect(nav.style.getPropertyValue('--at')).toBe('1');
  });
});
