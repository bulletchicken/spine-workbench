// @vitest-environment jsdom
import { createElement } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { find, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import type { DockSpec } from '@/guide/dock-spec';
import { dockNow, useDockNow, useShowInDock } from './dock-now';

const add: DockSpec = { scene: 'add', sentence: 'Add the four films.' };
const ready: DockSpec = { scene: 'ready', sentence: 'All four films are in.' };
function Caller({ spec }: { spec: DockSpec }) {
  useShowInDock(spec);
  return null;
}
function Reader() {
  return createElement('p', { id: 'dock-reading' }, useDockNow()?.sentence ?? '');
}

afterEach(() => { dockNow.getState().show(null); });

describe('the current dock', () => {
  it('keeps the last spec shown, including an empty dock', () => {
    dockNow.getState().show(add);
    dockNow.getState().show(ready);
    expect(dockNow.getState().spec).toBe(ready);
    dockNow.getState().show(null);
    expect(dockNow.getState().spec).toBeNull();
  });

  it('shows a caller’s changing spec and clears it when the caller unmounts', () => {
    const page = putOnThePage(createElement(Caller, { spec: add }));
    const reader = putOnThePage(createElement(Reader));
    expect(dockNow.getState().spec).toBe(add);
    expect(find('#dock-reading').textContent).toBe(add.sentence);
    page.show(createElement(Caller, { spec: ready }));
    expect(dockNow.getState().spec).toBe(ready);
    expect(find('#dock-reading').textContent).toBe(ready.sentence);
    page.show(null);
    expect(dockNow.getState().spec).toBeNull();
    expect(find('#dock-reading').textContent).toBe('');
    reader.show(null);
  });
});
