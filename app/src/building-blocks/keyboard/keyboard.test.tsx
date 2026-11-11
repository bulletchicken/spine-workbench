// The rules of the one key listener, tried with real key events in a
// document. The order of the ranks is written out here by hand, from the
// table the app was planned with, and not read from key-ranks.ts.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { find, focusOn, pressKey, putOnThePage } from '../test-rig/on-the-page';
import { stopKeysAt, takeKeys } from './keyboard';
import { useKey } from './use-key';

const PLANNED_ORDER = ['always', 'sheet', 'popover', 'gesture', 'choice', 'lit', 'review', 'pass', 'tools', 'cameras'] as const;

const letGo: (() => void)[] = [];
/** Register for this test only. */
function take(...args: Parameters<typeof takeKeys>): void {
  letGo.push(takeKeys(...args));
}
/** Let go of everything taken so far: after each test, pass or fail, and between the rounds of one. */
function letAllGo(): void {
  for (const stop of letGo.splice(0)) stop();
}
afterEach(letAllGo);

describe('the walk down the ranks', () => {
  it('asks every rank in the planned order, whatever order they registered in', () => {
    const asked: string[] = [];
    // Registered backwards and from the middle out, so that neither
    // registration order nor its reverse can pass for the planned order.
    for (const rank of ['lit', 'cameras', 'always', 'review', 'sheet', 'tools', 'gesture', 'pass', 'popover', 'choice'] as const) {
      take(rank, ['x'], () => {
        asked.push(rank);
        return false;
      });
    }
    const event = pressKey('x');
    expect(asked).toEqual([...PLANNED_ORDER]);
    expect(event.defaultPrevented).toBe(false);
  });

  it('stops at the first taker: a key taken at one rank never reaches a lower one', () => {
    PLANNED_ORDER.forEach((taking, at) => {
      const asked: string[] = [];
      for (const rank of PLANNED_ORDER) {
        take(rank, ['x'], () => {
          asked.push(rank);
          return rank === taking;
        });
      }
      const event = pressKey('x');
      expect(asked).toEqual(PLANNED_ORDER.slice(0, at + 1));
      expect(event.defaultPrevented).toBe(true);
      letAllGo();
    });
  });

  it('takes the key when run returns nothing at all', () => {
    const lower = vi.fn();
    take('lit', ['Enter'], () => undefined);
    take('pass', ['Enter'], lower);
    expect(pressKey('Enter').defaultPrevented).toBe(true);
    expect(lower).not.toHaveBeenCalled();
  });

  it('asks only the takers of the key that went down', () => {
    const v = vi.fn();
    const r = vi.fn();
    take('tools', ['v'], v);
    take('tools', ['r'], r);
    pressKey('r');
    expect(v).not.toHaveBeenCalled();
    expect(r).toHaveBeenCalledTimes(1);
  });

  it('within one rank asks the newest taker first', () => {
    const asked: string[] = [];
    take('sheet', ['Escape'], () => {
      asked.push('opened first');
    });
    take('sheet', ['Escape'], () => {
      asked.push('opened last');
    });
    pressKey('Escape');
    expect(asked).toEqual(['opened last']);
  });

  it('passes over a taker whose `when` says no', () => {
    let open = false;
    const close = vi.fn();
    const leaveReview = vi.fn();
    take('popover', ['Escape'], close, () => open);
    take('review', ['Escape'], leaveReview);
    pressKey('Escape');
    expect(close).not.toHaveBeenCalled();
    expect(leaveReview).toHaveBeenCalledTimes(1);
    open = true;
    pressKey('Escape');
    expect(close).toHaveBeenCalledTimes(1);
    expect(leaveReview).toHaveBeenCalledTimes(1);
  });

  it('matches letters in either case, and the space bar by the name Space', () => {
    const b = vi.fn();
    const space = vi.fn();
    take('tools', ['B'], b);
    take('pass', ['Space'], space);
    pressKey('b');
    pressKey('B', { shiftKey: true });
    pressKey(' ');
    expect(b).toHaveBeenCalledTimes(2);
    expect(space).toHaveBeenCalledTimes(1);
  });
});

describe('the collisions the order settles', () => {
  it('B answers the choice on screen and does not pick up the boundary wand', () => {
    const pickBoundaryWand = vi.fn();
    const answerB = vi.fn();
    take('tools', ['b'], pickBoundaryWand);
    take('choice', ['a', 'b'], answerB);
    pressKey('b');
    expect(answerB).toHaveBeenCalledTimes(1);
    expect(pickBoundaryWand).not.toHaveBeenCalled();
  });

  it('Escape in the middle of a drag drops the drag and the review stays; with no drag it leaves the review', () => {
    let dragging = true;
    const leaveReview = vi.fn();
    take('review', ['Escape'], leaveReview);
    take('gesture', ['Escape'], () => {
      if (!dragging) return false;
      dragging = false;
      return true;
    });
    pressKey('Escape');
    expect(dragging).toBe(false);
    expect(leaveReview).not.toHaveBeenCalled();
    pressKey('Escape');
    expect(leaveReview).toHaveBeenCalledTimes(1);
  });

  it('Escape closes an open dialog before the popover behind it', () => {
    const closed: string[] = [];
    take('popover', ['Escape'], () => {
      closed.push('popover');
    });
    take('sheet', ['Escape'], () => {
      closed.push('dialog');
    });
    pressKey('Escape');
    expect(closed).toEqual(['dialog']);
  });
});

describe('who is asked at all', () => {
  it('a key held with Cmd, Ctrl or Alt reaches always and nothing else', () => {
    for (const held of [{ metaKey: true }, { ctrlKey: true }, { altKey: true }]) {
      const asked: string[] = [];
      for (const rank of PLANNED_ORDER) {
        take(rank, ['z'], () => {
          asked.push(rank);
          return false;
        });
      }
      pressKey('z', held);
      expect(asked).toEqual(['always']);
      letAllGo();
    }
  });

  it('Shift alone is not such a key: + is Shift and = on most keyboards', () => {
    const zoomIn = vi.fn();
    take('cameras', ['+'], zoomIn);
    pressKey('+', { shiftKey: true });
    expect(zoomIn).toHaveBeenCalledTimes(1);
  });

  it('nothing but always runs while typing', () => {
    putOnThePage(
      <>
        <input aria-label="length" />
        <input type="number" aria-label="millimetres" />
        <input type="search" aria-label="find a level" />
        <textarea aria-label="note" />
        <select aria-label="unit">
          <option>mm</option>
        </select>
        <div contentEditable suppressContentEditableWarning data-testid="rich" tabIndex={0} />
      </>,
    );
    for (const field of ['input:not([type])', 'input[type="number"]', 'input[type="search"]', 'textarea', 'select', '[data-testid="rich"]']) {
      const asked: string[] = [];
      for (const rank of PLANNED_ORDER) {
        take(rank, ['b', 'Enter', 'Escape', 'Space'], () => {
          asked.push(rank);
          return false;
        });
      }
      focusOn(find(field));
      for (const key of ['b', 'Enter', 'Escape', ' ']) pressKey(key);
      expect(asked, field).toEqual(['always', 'always', 'always', 'always']);
      letAllGo();
    }
  });

  it('a checkbox, a radio button and a range are not typed into: Escape and a letter on them walk the ranks', () => {
    putOnThePage(
      <>
        <input type="checkbox" aria-label="keep the old outline" />
        <input type="radio" aria-label="left" />
        <input type="range" aria-label="brightness" />
      </>,
    );
    const closePopover = vi.fn();
    const pickBoundaryWand = vi.fn();
    take('popover', ['Escape'], closePopover);
    take('tools', ['b'], pickBoundaryWand);
    for (const control of ['checkbox', 'radio', 'range']) {
      focusOn(find(`input[type="${control}"]`));
      expect(pressKey('Escape').defaultPrevented, control).toBe(true);
      expect(pressKey('b').defaultPrevented, control).toBe(true);
    }
    expect(closePopover).toHaveBeenCalledTimes(3);
    expect(pickBoundaryWand).toHaveBeenCalledTimes(3);
  });

  it('but each keeps the keys the browser answers on it', () => {
    const playPass = vi.fn();
    const answerLeft = vi.fn();
    const pressLit = vi.fn();
    const goHome = vi.fn();
    take('pass', ['Space'], playPass);
    take('choice', ['ArrowLeft'], answerLeft);
    take('lit', ['Enter'], pressLit);
    take('cameras', ['Home'], goHome);
    putOnThePage(
      <>
        <input type="checkbox" aria-label="keep the old outline" />
        <input type="radio" aria-label="left" />
        <input type="range" aria-label="brightness" />
        <input type="submit" value="Save" />
      </>,
    );
    // Written out by hand: which of Space, ←, Home and Enter the browser itself answers on each control.
    const KEPT: Record<string, readonly string[]> = {
      checkbox: [' '],
      radio: [' ', 'ArrowLeft'],
      range: ['ArrowLeft', 'Home'],
      submit: [' ', 'Enter'],
    };
    for (const [control, kept] of Object.entries(KEPT)) {
      focusOn(find(`input[type="${control}"]`));
      for (const key of [' ', 'ArrowLeft', 'Home', 'Enter']) {
        expect(pressKey(key).defaultPrevented, `${key === ' ' ? 'Space' : key} on a ${control}`).toBe(!kept.includes(key));
      }
    }
    // Each taker was reached once for every control that does not keep its key.
    expect(playPass).toHaveBeenCalledTimes(1);
    expect(answerLeft).toHaveBeenCalledTimes(2);
    expect(goHome).toHaveBeenCalledTimes(3);
    expect(pressLit).toHaveBeenCalledTimes(3);
  });

  it('Enter and Space on a focused button or link are that control’s own', () => {
    const pressLit = vi.fn();
    const playPass = vi.fn();
    take('lit', ['Enter'], pressLit);
    take('pass', ['Space'], playPass);
    putOnThePage(
      <>
        <button type="button">Skip for now</button>
        <a href="#levels">Levels</a>
        <div role="button" tabIndex={0} data-testid="slot" />
        <p tabIndex={0}>a paragraph</p>
      </>,
    );
    for (const control of ['button', 'a', '[data-testid="slot"]']) {
      focusOn(find(control));
      expect(pressKey('Enter').defaultPrevented, control).toBe(false);
      expect(pressKey(' ').defaultPrevented, control).toBe(false);
    }
    expect(pressLit).not.toHaveBeenCalled();
    expect(playPass).not.toHaveBeenCalled();

    // Anything else in focus leaves Enter to the lit button.
    focusOn(find('p'));
    pressKey('Enter');
    pressKey(' ');
    expect(pressLit).toHaveBeenCalledTimes(1);
    expect(playPass).toHaveBeenCalledTimes(1);
  });

  it('other keys on a focused button still walk: B on a focused button answers the choice', () => {
    const answerB = vi.fn();
    take('choice', ['b'], answerB);
    putOnThePage(<button type="button">Skip for now</button>);
    focusOn(find('button'));
    pressKey('b');
    expect(answerB).toHaveBeenCalledTimes(1);
  });

  it('a modified key on a focused button still reaches always', () => {
    const undo = vi.fn();
    take('always', ['z'], undo);
    putOnThePage(<button type="button">Skip for now</button>);
    focusOn(find('button'));
    pressKey('z', { metaKey: true });
    expect(undo).toHaveBeenCalledTimes(1);
  });

  it('a key a focused control handled itself never starts the walk', () => {
    const answerLeft = vi.fn();
    take('choice', ['ArrowLeft'], answerLeft);
    putOnThePage(
      <div
        role="slider"
        aria-valuenow={0}
        tabIndex={0}
        onKeyDown={(event) => {
          if (event.key === 'ArrowLeft') event.preventDefault();
        }}
      />,
    );
    focusOn(find('[role="slider"]'));
    pressKey('ArrowLeft');
    expect(answerLeft).not.toHaveBeenCalled();
  });

  it('leaves a key alone while text is being composed', () => {
    const pressLit = vi.fn();
    take('lit', ['Enter'], pressLit);
    pressKey('Enter', { isComposing: true });
    expect(pressLit).not.toHaveBeenCalled();
  });
});

describe('keys stopped at a rank', () => {
  /** Every rank asked for `x`, in the order asked; none takes it. */
  function askEveryRank(): string[] {
    const asked: string[] = [];
    for (const rank of PLANNED_ORDER) {
      take(rank, ['x'], () => {
        asked.push(rank);
        return false;
      });
    }
    return asked;
  }
  /** Stop the keys for this test only; returns the way to let them go sooner. */
  function stopAt(rank: (typeof PLANNED_ORDER)[number]): () => void {
    const letThrough = stopKeysAt(rank);
    letGo.push(letThrough);
    return letThrough;
  }

  it('asks no rank below it, and leaves a key nobody took its default', () => {
    const asked = askEveryRank();
    const letThrough = stopAt('sheet');
    const event = pressKey('x');
    expect(asked).toEqual(['always', 'sheet']);
    expect(event.defaultPrevented).toBe(false);

    letThrough();
    asked.length = 0;
    pressKey('x');
    expect(asked).toEqual([...PLANNED_ORDER]);
  });

  it('still lets a taker at that rank, or above it, take the key', () => {
    const closeDialog = vi.fn();
    const undo = vi.fn();
    const leaveReview = vi.fn();
    take('sheet', ['Escape'], closeDialog);
    take('always', ['z'], undo);
    take('review', ['Escape'], leaveReview);
    stopAt('sheet');
    expect(pressKey('Escape').defaultPrevented).toBe(true);
    pressKey('z', { metaKey: true });
    expect(closeDialog).toHaveBeenCalledTimes(1);
    expect(undo).toHaveBeenCalledTimes(1);
    expect(leaveReview).not.toHaveBeenCalled();
  });

  it('with two parts stopping the keys, the higher rank holds until it lets go', () => {
    const asked = askEveryRank();
    stopAt('choice');
    const letSheetGo = stopAt('sheet');
    pressKey('x');
    expect(asked).toEqual(['always', 'sheet']);

    letSheetGo();
    asked.length = 0;
    pressKey('x');
    expect(asked).toEqual(['always', 'sheet', 'popover', 'gesture', 'choice']);
  });

  it('letting go twice lets go once: another part stopping at the same rank still holds', () => {
    const asked = askEveryRank();
    const first = stopAt('sheet');
    stopAt('sheet');
    first();
    first();
    pressKey('x');
    expect(asked).toEqual(['always', 'sheet']);
  });
});

describe('useKey', () => {
  function Lit({ press, live = true }: { press: () => void; live?: boolean }) {
    useKey('lit', ['Enter'], press, () => live);
    return null;
  }

  it('answers while its part is mounted and is gone when the part unmounts', () => {
    const press = vi.fn();
    const page = putOnThePage(<Lit press={press} />);
    pressKey('Enter');
    expect(press).toHaveBeenCalledTimes(1);
    page.takeDown();
    const event = pressKey('Enter');
    expect(press).toHaveBeenCalledTimes(1);
    expect(event.defaultPrevented).toBe(false);
  });

  it('runs the newest function it was given, and asks the newest `when`', () => {
    const first = vi.fn();
    const second = vi.fn();
    const page = putOnThePage(<Lit press={first} />);
    page.show(<Lit press={second} />);
    pressKey('Enter');
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);
    page.show(<Lit press={second} live={false} />);
    pressKey('Enter');
    expect(second).toHaveBeenCalledTimes(1);
  });

  it('keeps its place among the takers of its rank when it renders again', () => {
    const asked: string[] = [];
    function Taker({ name, tick }: { name: string; tick: number }) {
      useKey('sheet', ['Escape'], () => {
        asked.push(`${name} ${String(tick)}`);
        return false;
      });
      return null;
    }
    // Each from a root of its own, so the older one can render again by itself:
    // if that registered it afresh, it would be asked first.
    const older = putOnThePage(<Taker name="older" tick={0} />);
    putOnThePage(<Taker name="newer" tick={0} />);
    older.show(<Taker name="older" tick={1} />);
    pressKey('Escape');
    expect(asked).toEqual(['newer 0', 'older 1']);
  });

  it('listens on the document once, however many parts register, and stops with the last', () => {
    const add = vi.spyOn(document, 'addEventListener');
    const remove = vi.spyOn(document, 'removeEventListener');
    const page = putOnThePage(
      <>
        <Lit press={vi.fn()} />
        <Lit press={vi.fn()} />
        <Lit press={vi.fn()} />
      </>,
    );
    expect(add.mock.calls.filter(([type]) => type === 'keydown')).toHaveLength(1);
    expect(remove.mock.calls.filter(([type]) => type === 'keydown')).toHaveLength(0);
    page.takeDown();
    expect(remove.mock.calls.filter(([type]) => type === 'keydown')).toHaveLength(1);
    add.mockRestore();
    remove.mockRestore();
  });
});
