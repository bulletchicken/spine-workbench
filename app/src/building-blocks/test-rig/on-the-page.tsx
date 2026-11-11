// For component tests: a part mounted in the test's document, and real
// DOM events sent to it the way a browser sends them. Imported only by
// *.test.tsx files, which run in jsdom.

import { act, type ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach } from 'vitest';

declare global {
  // React reads this to know its updates are wrapped in act().
  var IS_REACT_ACT_ENVIRONMENT: boolean | undefined;
}
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

const mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of mounted.splice(0)) {
    act(() => {
      root.unmount();
    });
    host.remove();
  }
});

/** Mount `node` in the document. Everything mounted is taken down after each test. */
export function putOnThePage(node: ReactNode): { host: HTMLElement; show(next: ReactNode): void; takeDown(): void } {
  const host = document.createElement('div');
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({ root, host });
  const show = (next: ReactNode) => {
    act(() => {
      root.render(next);
    });
  };
  show(node);
  return {
    host,
    show,
    takeDown: () => {
      act(() => {
        root.unmount();
      });
    },
  };
}

/** The one element matching `selector`; the test fails if there is none. */
export function find(selector: string, within: ParentNode = document): HTMLElement {
  const found = within.querySelector<HTMLElement>(selector);
  if (found === null) throw new Error(`nothing on the page matches ${selector}`);
  return found;
}

/**
 * A key going down on the focused element (or `on`), as a real event that
 * bubbles to the document. Returns the event, so a test can ask whether
 * its default was prevented.
 */
export function pressKey(key: string, init: KeyboardEventInit = {}, on: Element = document.activeElement ?? document.body): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
  act(() => {
    on.dispatchEvent(event);
  });
  return event;
}

/** A press of the main pointer button on `on`: pointer down and up, then the click. */
export function pressOn(on: Element): void {
  act(() => {
    on.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, cancelable: true }));
    on.dispatchEvent(new MouseEvent('pointerup', { bubbles: true, cancelable: true }));
    on.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  });
}

/** Put focus on `on`, as a script does. */
export function focusOn(on: HTMLElement): void {
  act(() => {
    on.focus();
  });
}

/**
 * Put focus on `on` as the Tab key does, so that it is `:focus-visible`:
 * jsdom, like a browser, shows keyboard focus only after a key.
 */
export function tabTo(on: HTMLElement): void {
  act(() => {
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }));
    on.focus();
  });
}

/** Send any other event to `on`. */
export function send(on: Element, event: Event): void {
  act(() => {
    on.dispatchEvent(event);
  });
}

/** The pointer coming to rest on `on`, and leaving it again. React reads entering and leaving from these two events. */
export function pointerOnto(on: Element): void {
  act(() => {
    on.dispatchEvent(new MouseEvent('pointerover', { bubbles: true, relatedTarget: null }));
  });
}

export function pointerOff(on: Element): void {
  act(() => {
    on.dispatchEvent(new MouseEvent('pointerout', { bubbles: true, relatedTarget: document.body }));
  });
}

/**
 * Tell `on` that its CSS animation has ended. jsdom runs no animations, so
 * a part that waits for one to end is told so by hand. Both spellings are
 * sent, because React listens for the prefixed one where the environment
 * has no AnimationEvent, as jsdom has not.
 */
export function endAnimation(on: Element): void {
  act(() => {
    on.dispatchEvent(new Event('animationend', { bubbles: true }));
    on.dispatchEvent(new Event('webkitAnimationEnd', { bubbles: true }));
  });
}
