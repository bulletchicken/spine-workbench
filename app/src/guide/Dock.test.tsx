import { act } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { find, focusOn, pressKey, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { Dock, STILL_AFTER_MS } from './Dock';
import type { DockSpec } from './dock-spec';

const nothing = () => undefined;
const add = (press = nothing): DockSpec => ({
  scene: 'add', sentence: 'Add the four films.',
  helper: { kind: 'words', words: ['Drop them anywhere, or ', { link: 'choose them', press } satisfies { link: string; press: () => void }, '.'] },
  quiet: { label: 'Use the sample case', press },
  primary: { label: 'Find the spine', press },
});
const found = (): DockSpec => ({ scene: 'found', sentence: '17 vertebrae found.', helper: { kind: 'look', words: 'T1 is marked on the films.' } });

/** Watch actual DOM changes, so an unchanged live region really stays silent. */
function mutationsDuring(change: () => void): MutationRecord[] {
  const watch = new MutationObserver(nothing);
  watch.observe(find('[role="status"]'), { childList: true, subtree: true, characterData: true });
  change();
  const changes = watch.takeRecords();
  watch.disconnect();
  return changes;
}

describe('the dock', () => {
  it('keeps all scenes in order, with only the current one on and the others inert', () => {
    const page = putOnThePage(<Dock spec={add()} />);
    const scenes = [...document.querySelectorAll('.scene')];
    expect(scenes.map((scene) => scene.getAttribute('data-scene'))).toEqual(['add', 'reading', 'ready', 'finding', 'found', 'review', 'failed', 'engine', 'building', 'build-failed', 'three', 'compare']);
    expect(document.querySelectorAll('.scene.on')).toHaveLength(1);
    for (const scene of scenes) {
      expect(scene.hasAttribute('inert')).toBe(!scene.classList.contains('on'));
      expect(scene.getAttribute('aria-hidden')).toBe(scene.classList.contains('on') ? null : 'true');
    }
    page.show(<Dock spec={null} />);
    expect(document.querySelectorAll('.scene')).toHaveLength(12);
    expect(document.querySelector('.scene.on')).toBeNull();
    expect(find('[role="status"]').textContent).toBe('');
  });

  it('keeps a leaving scene’s words and buttons for its fade', () => {
    const page = putOnThePage(<Dock spec={add()} />);
    const sentence = find('[data-scene="add"] h2');
    const primary = find('[data-scene="add"] .primary');
    page.show(<Dock spec={found()} />);
    expect(find('[data-scene="add"] h2')).toBe(sentence);
    expect(sentence.textContent).toBe('Add the four films.');
    expect(find('[data-scene="add"] .primary')).toBe(primary);
    expect(find('.scene.on').getAttribute('data-scene')).toBe('found');
    expect(find('.dot').getAttribute('aria-hidden')).toBe('true');
  });

  it('speaks the current sentence and helper, with no button, and leaves the line alone when words match', () => {
    const page = putOnThePage(<Dock spec={add()} />);
    const status = find('[role="status"]');
    expect(status.className).toBe('spoken-only');
    expect(status.textContent).toBe('Add the four films. Drop them anywhere, or choose them.');
    expect(status.querySelector('button')).toBeNull();
    expect(document.querySelectorAll('[role="status"], [aria-live]')).toHaveLength(1);
    expect(mutationsDuring(() => { page.show(<Dock spec={add()} />); })).toEqual([]);
    page.show(<Dock spec={found()} />);
    expect(status.textContent).toBe('17 vertebrae found. T1 is marked on the films.');
  });

  it('changes counts and progress without speaking them, and starts a new meter when a count replaces the wait', () => {
    const page = putOnThePage(<Dock spec={{ scene: 'finding', sentence: 'Finding the spine…', helper: { kind: 'meter', share: 0.8, words: 'Searching the Front film.' } }} />);
    const wait = find('[role="progressbar"]');
    expect(wait.getAttribute('aria-valuenow')).toBe('80');
    expect(wait.getAttribute('aria-valuetext')).toBe('80%');
    page.show(<Dock spec={{ scene: 'finding', sentence: 'Finding the spine…', helper: { kind: 'count', count: '0 of 17', spoken: '0 of 17', share: 0 } }} />);
    const count = find('[role="progressbar"]');
    expect(count).not.toBe(wait);
    expect(count.style.getPropertyValue('--p')).toBe('0');
    expect(count.getAttribute('aria-label')).toBe('Finding the spine…');
    expect(count.getAttribute('aria-valuemin')).toBe('0');
    expect(count.getAttribute('aria-valuemax')).toBe('100');
    expect(find('.num').getAttribute('aria-hidden')).toBe('true');
    expect(mutationsDuring(() => {
      page.show(<Dock spec={{ scene: 'finding', sentence: 'Finding the spine…', helper: { kind: 'count', count: '9 of 17', spoken: '9 of 17', share: 9 / 17 } }} />);
    })).toEqual([]);
    expect(find('[role="progressbar"]')).toBe(count);
    expect(count.getAttribute('aria-valuetext')).toBe('9 of 17');
    expect(count.getAttribute('aria-valuenow')).toBe('53');
    expect(count.firstElementChild?.tagName).toBe('I');
    expect(find('[role="status"]').textContent).toBe('Finding the spine…');
  });

  it('shows that work goes on when a wait’s meter has stood still, and never for a count or a scene that has left', () => {
    vi.useFakeTimers();
    try {
      const wait = (share: number): DockSpec => ({ scene: 'finding', sentence: 'Finding the spine…', helper: { kind: 'meter', share, words: 'Searching 4 films.' } });
      const page = putOnThePage(<Dock spec={wait(0.3)} />);
      const meter = () => find('[data-scene="finding"] [role="progressbar"]');
      act(() => { vi.advanceTimersByTime(STILL_AFTER_MS - 1); });
      expect(meter().className).toBe('meter');
      act(() => { vi.advanceTimersByTime(1); });
      expect(meter().className).toBe('meter waiting');
      // A step forward is news: the plain bar is back at once, and the watch starts again.
      page.show(<Dock spec={wait(0.5)} />);
      expect(meter().className).toBe('meter');
      act(() => { vi.advanceTimersByTime(STILL_AFTER_MS); });
      expect(meter().className).toBe('meter waiting');
      // A count stands still because nothing new has been counted.
      page.show(<Dock spec={{ scene: 'finding', sentence: 'Finding the spine…', helper: { kind: 'count', count: '3 of 17', spoken: '3 of 17', share: 3 / 17 } }} />);
      act(() => { vi.advanceTimersByTime(10 * STILL_AFTER_MS); });
      expect(meter().className).toBe('meter');
      // The wait's scene has left: its kept meter is not watched.
      page.show(<Dock spec={wait(0.7)} />);
      page.show(<Dock spec={found()} />);
      act(() => { vi.advanceTimersByTime(10 * STILL_AFTER_MS); });
      expect(meter().className).toBe('meter');
    } finally {
      vi.useRealTimers();
    }
  });

  it('runs the link, quiet action and primary only in the scene that is on', () => {
    const press = vi.fn();
    const page = putOnThePage(<Dock spec={add(press)} />);
    const link = find('.link');
    const quiet = find('.quiet');
    const primary = find('.primary');
    for (const button of [link, quiet, primary]) pressOn(button);
    expect(press).toHaveBeenCalledTimes(3);
    page.show(<Dock spec={found()} />);
    for (const button of [link, quiet, primary]) pressOn(button);
    expect(press).toHaveBeenCalledTimes(3);
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    expect(press).toHaveBeenCalledTimes(3);
  });

  it('takes Enter once for the current primary, and gives it up while disabled or absent', () => {
    const press = vi.fn();
    const page = putOnThePage(<Dock spec={add(press)} />);
    expect(pressKey('Enter').defaultPrevented).toBe(true);
    pressKey('Enter', { repeat: true });
    expect(press).toHaveBeenCalledTimes(1);
    page.show(<Dock spec={{ scene: 'add', sentence: 'Add the four films.', primary: { label: 'Find the spine', press, disabled: true } }} />);
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    pressOn(find('.scene.on .primary'));
    expect(press).toHaveBeenCalledTimes(1);
    page.show(<Dock spec={found()} />);
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    page.show(<Dock spec={null} />);
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    expect(press).toHaveBeenCalledTimes(1);
  });

  it('leaves Enter to a focused quiet action', () => {
    const press = vi.fn();
    putOnThePage(<Dock spec={add(press)} />);
    focusOn(find('.quiet'));
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    expect(press).not.toHaveBeenCalled();
    pressOn(find('.quiet'));
    expect(press).toHaveBeenCalledTimes(1);
  });

  it('keeps buttons and their focus within a scene while new words arrive', () => {
    const press = vi.fn();
    const page = putOnThePage(<Dock spec={add()} />);
    const quiet = find('.quiet');
    const primary = find('.primary');
    const sentence = find('.scene.on h2 span');
    const helper = find('.scene.on p [data-fresh]');
    focusOn(quiet);
    page.show(<Dock spec={{ ...add(press), sentence: 'The Front film is in.', helper: { kind: 'words', words: ['Each one is in its place.'] }, quiet: { label: 'Start over', press } }} />);
    expect(find('.quiet')).toBe(quiet);
    expect(find('.primary')).toBe(primary);
    expect(document.activeElement).toBe(quiet);
    expect(quiet.textContent).toBe('Start over');
    expect(find('.scene.on h2 span')).not.toBe(sentence);
    expect(find('.scene.on p [data-fresh]')).not.toBe(helper);
    pressOn(quiet);
    expect(press).toHaveBeenCalledTimes(1);
  });
});
