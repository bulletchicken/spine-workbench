// The guide's promises: Enter presses the lit button and nothing else; a
// choice is answered once, by its own keys, and never by Enter; and the
// words are spoken once, because they are drawn afresh only when they read
// differently.

import { describe, expect, it, vi } from 'vitest';
import { find, focusOn, pointerOff, pointerOnto, pressKey, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { GuideCard } from './cards/GuideCard';
import { Guide } from './Guide';
import type { Choice, GuideSpec } from './guide-spec';
import { LitStandsDown } from './lit-stands-down';

const lit = (press: () => void, quiet: () => void = () => undefined): GuideSpec => ({
  eyebrow: ['Step 1 of 4 · Films'],
  sentence: 'Hang your four films.',
  body: ['Drop the files anywhere on this window, or choose them.'],
  lit: { label: 'Choose films', press },
  quiet: [{ label: 'Open a saved case', press: quiet }],
});

const asked = (left: () => void, right: () => void, more: Partial<GuideSpec> = {}): GuideSpec => ({
  eyebrow: ['Step 2 of 4 · Size & facing · Front film'],
  sentence: 'Which side is the patient’s left?',
  choices: [
    { id: 'left', keys: ['ArrowLeft'], title: 'This side', line: 'Left of the picture', choose: left },
    { id: 'right', keys: ['ArrowRight'], title: 'This side', line: 'Right of the picture', tag: 'marker', choose: right },
  ],
  ...more,
});

describe('the lit button', () => {
  it('is pressed by Enter from anywhere on the page, and by a click', () => {
    const press = vi.fn();
    putOnThePage(<Guide spec={lit(press)} />);
    expect(pressKey('Enter').defaultPrevented).toBe(true);
    expect(press).toHaveBeenCalledTimes(1);
    pressOn(find('.lit-button'));
    expect(press).toHaveBeenCalledTimes(2);
  });

  it('is pressed once by Enter held down', () => {
    const press = vi.fn();
    putOnThePage(<Guide spec={lit(press)} />);
    pressKey('Enter');
    pressKey('Enter', { repeat: true });
    pressKey('Enter', { repeat: true });
    expect(press).toHaveBeenCalledTimes(1);
  });

  it('is left alone when Enter is pressed on a focused quiet button, which is pressed instead', () => {
    const press = vi.fn();
    const quiet = vi.fn();
    putOnThePage(<Guide spec={lit(press, quiet)} />);
    const button = [...document.querySelectorAll<HTMLElement>('.quiet-actions button')].find((each) => each.textContent === 'Open a saved case');
    if (button === undefined) throw new Error('the quiet action is not on the page');
    focusOn(button);
    // The browser turns Enter on a focused button into its click; the key itself is nobody else's.
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    pressOn(button);
    expect(press).not.toHaveBeenCalled();
    expect(quiet).toHaveBeenCalledTimes(1);
  });

  it('stands down to an outline, and gives up Enter, while a sheet has the lit button', () => {
    const press = vi.fn();
    putOnThePage(
      <LitStandsDown value>
        <Guide spec={lit(press)} />
      </LitStandsDown>,
    );
    expect(find('.lit-button').hasAttribute('data-stood-down')).toBe(true);
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    expect(press).not.toHaveBeenCalled();
  });

  it('shows the Enter key, and the icon it is given', () => {
    putOnThePage(<Guide spec={{ ...lit(() => undefined), lit: { label: 'Split T12 at the gap', icon: 'boundary-wand', press: () => undefined } }} />);
    expect(find('.lit-button kbd').textContent).toBe('Enter');
    expect(document.querySelector('.lit-button svg')).not.toBeNull();
    expect(find('.lit-button').textContent).toContain('Split T12 at the gap');
  });
});

describe('a pair of choices', () => {
  it('is answered exactly once by its own key, and not again while the key is held', () => {
    const left = vi.fn();
    const right = vi.fn();
    putOnThePage(<Guide spec={asked(left, right)} />);
    expect(pressKey('ArrowLeft').defaultPrevented).toBe(true);
    pressKey('ArrowLeft', { repeat: true });
    expect(left).toHaveBeenCalledTimes(1);
    expect(right).not.toHaveBeenCalled();
    pressKey('ArrowRight');
    expect(right).toHaveBeenCalledTimes(1);
  });

  it('is answered by a letter in either case', () => {
    const a = vi.fn();
    const b = vi.fn();
    const choices: [Choice, Choice] = [
      { id: 'a', keys: ['a'], title: 'The upper one', line: 'Counted up from the sacrum', choose: a },
      { id: 'b', keys: ['b'], title: 'The lower one', line: 'Counted down from the first rib', choose: b },
    ];
    putOnThePage(<Guide spec={{ eyebrow: ['Needs you'], sentence: 'Which vertebra is L5?', choices }} />);
    pressKey('B');
    pressKey('a');
    expect(a).toHaveBeenCalledTimes(1);
    expect(b).toHaveBeenCalledTimes(1);
    expect([...document.querySelectorAll('.choice-card-key')].map((key) => key.textContent)).toEqual(['A', 'B']);
  });

  it('has no default: Enter answers nothing, and no lit button is drawn beside it', () => {
    const left = vi.fn();
    const right = vi.fn();
    putOnThePage(<Guide spec={asked(left, right, { lit: { label: 'Never drawn', press: left } })} />);
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    expect(left).not.toHaveBeenCalled();
    expect(right).not.toHaveBeenCalled();
    expect(document.querySelector('.lit-button')).toBeNull();
  });

  it('says which card the pointer or the focus is on, and lights the card it is told to', () => {
    const hot = vi.fn();
    const nothing = () => undefined;
    const page = putOnThePage(<Guide spec={asked(nothing, nothing, { onHotChoice: hot })} />);
    const [first, second] = document.querySelectorAll<HTMLElement>('.choice-card');
    if (first === undefined || second === undefined) throw new Error('two cards are expected');

    pointerOnto(second);
    expect(hot).toHaveBeenLastCalledWith('right');
    pointerOff(second);
    expect(hot).toHaveBeenLastCalledWith(null);
    focusOn(first);
    expect(hot).toHaveBeenLastCalledWith('left');

    expect(second.hasAttribute('data-hot')).toBe(false);
    page.show(<Guide spec={asked(nothing, nothing, { onHotChoice: hot, hotChoice: 'right' })} />);
    expect(second.hasAttribute('data-hot')).toBe(true);
    expect(first.hasAttribute('data-hot')).toBe(false);
    // Lighting a card draws nothing afresh: the card with focus is still the same card, still focused.
    expect(document.activeElement).toBe(first);
    expect(second.querySelector('.choice-card-tag')?.textContent).toBe('marker');
  });
});

describe('the words', () => {
  /** What changed inside the live region while `change` ran. */
  function changesDuring(change: () => void): MutationRecord[] {
    const region = find('[aria-live="polite"]');
    const watch = new MutationObserver(() => undefined);
    watch.observe(region, { childList: true, subtree: true, characterData: true });
    change();
    const seen = watch.takeRecords();
    watch.disconnect();
    return seen;
  }

  it('are a polite live region holding the eyebrow, the sentence and the paragraph, and no button', () => {
    putOnThePage(<Guide spec={lit(() => undefined)} />);
    const region = find('[aria-live="polite"]');
    expect(region.getAttribute('aria-atomic')).toBe('true');
    expect(region.textContent).toBe('Step 1 of 4 · FilmsHang your four films.Drop the files anywhere on this window, or choose them.');
    expect(region.querySelector('button')).toBeNull();
    expect(find('h1').textContent).toBe('Hang your four films.');
  });

  it('are not touched by a render that says the same, so nothing is spoken twice', () => {
    const nothing = () => undefined;
    const page = putOnThePage(<Guide spec={asked(nothing, nothing)} />);
    const region = find('[aria-live="polite"]');
    const sentence = find('h1');

    // The same words from a new spec with new functions, and a card lit from the film.
    expect(
      changesDuring(() => {
        page.show(<Guide spec={asked(() => undefined, () => undefined)} />);
        page.show(<Guide spec={asked(nothing, nothing, { hotChoice: 'left' })} />);
      }),
    ).toEqual([]);
    expect(find('h1')).toBe(sentence);

    // A new sentence is drawn afresh, once, inside the same region.
    const changes = changesDuring(() => {
      page.show(<Guide spec={lit(nothing)} />);
    });
    expect(changes.length).toBeGreaterThan(0);
    expect(find('[aria-live="polite"]')).toBe(region);
    expect(find('h1')).not.toBe(sentence);
    expect(document.querySelectorAll('h1')).toHaveLength(1);
  });

  it('stay as they are when only what is offered changes', () => {
    const nothing = () => undefined;
    const page = putOnThePage(<Guide spec={lit(nothing)} />);
    const button = find('.lit-button');
    expect(
      changesDuring(() => {
        page.show(<Guide spec={{ ...lit(nothing), lit: { label: 'Choose another file', press: nothing } }} />);
      }),
    ).toEqual([]);
    // What is offered is new, so it arrives afresh.
    expect(find('.lit-button')).not.toBe(button);
    expect(find('.lit-button').textContent).toContain('Choose another file');
  });

  it('keep a quiet action that is offered again as the same button, with its focus', () => {
    const nothing = () => undefined;
    const skip = { label: 'Skip for now', press: nothing };
    const page = putOnThePage(<Guide spec={{ ...lit(nothing), quiet: [{ label: 'It really is one', press: nothing }, skip] }} />);
    const button = (label: string) => [...document.querySelectorAll<HTMLElement>('.quiet-actions button')].find((each) => each.textContent === label);
    const skipButton = button('Skip for now');
    if (skipButton === undefined) throw new Error('Skip for now is not on the page');
    focusOn(skipButton);

    page.show(
      <Guide
        spec={{
          eyebrow: ['Needs you'],
          sentence: 'T3 is hard to see.',
          lit: { label: 'Add the missed bone', press: nothing },
          quiet: [{ label: 'It looks right', press: nothing }, skip],
        }}
      />,
    );
    expect(button('Skip for now')).toBe(skipButton);
    expect(document.activeElement).toBe(skipButton);
    expect(button('It really is one')).toBeUndefined();
    expect(button('It looks right')).toBeDefined();
  });

  it('set strong words strong', () => {
    putOnThePage(<Guide spec={{ eyebrow: ['Needs you · ', { strong: '2 of 4' }], sentence: 'One vertebra is missing.', body: ['It is ', { strong: 'T12' }, '.'] }} />);
    expect(find('.guide-eyebrow b').textContent).toBe('2 of 4');
    expect(find('.guide-body').textContent).toBe('It is T12.');
    expect(find('.guide-body b').textContent).toBe('T12');
  });
});

describe('arriving', () => {
  const place = (element: Element) => (element as HTMLElement).style.getPropertyValue('--i');

  it('gives each line its place, top to bottom', () => {
    putOnThePage(
      <Guide spec={lit(() => undefined)}>
        <GuideCard heading="Films" count="0 of 4">
          rows
        </GuideCard>
      </Guide>,
    );
    expect([...document.querySelectorAll('.guide-line, .guide-quiet')].map(place)).toEqual(['0', '1', '2', '3', '4']);
    expect(find('.guide-card-count').textContent).toBe('0 of 4');
  });

  it('does not draw a card afresh when it stays while the words and the card above it change', () => {
    const nothing = () => undefined;
    const page = putOnThePage(
      <Guide spec={lit(nothing)}>
        <>
          <GuideCard heading="What it is doing">lines</GuideCard>
          <GuideCard heading="How sure">cells</GuideCard>
        </>
      </Guide>,
    );
    const [doing, howSure] = document.querySelectorAll('.guide-card');
    const Checks = () => <GuideCard heading="Checks">rows</GuideCard>;
    page.show(
      <Guide spec={asked(nothing, nothing)}>
        <>
          <Checks />
          <GuideCard heading="How sure">cells</GuideCard>
        </>
      </Guide>,
    );
    const cards = [...document.querySelectorAll('.guide-card')];
    expect(cards).toHaveLength(2);
    // The card that changed is new; the one under it is the very same element.
    expect(cards[0]).not.toBe(doing);
    expect(cards[1]).toBe(howSure);
  });

  it('shows at most two quiet actions', () => {
    const nothing = () => undefined;
    putOnThePage(
      <Guide
        spec={{
          ...lit(nothing),
          quiet: [
            { label: 'One', press: nothing },
            { label: 'Two', press: nothing },
            { label: 'Three', press: nothing },
          ],
        }}
      />,
    );
    expect([...document.querySelectorAll('.quiet-actions button')].map((button) => button.textContent)).toEqual(['One', 'Two']);
  });
});
