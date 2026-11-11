import { afterEach, describe, expect, it, vi } from 'vitest';
import { takeKeys } from './keyboard/keyboard';
import { Slider } from './Slider';
import type { SliderSegment } from './slider-geometry';
import { find, focusOn, pressKey, putOnThePage, send } from './test-rig/on-the-page';

// The six beats of the pass, in milliseconds.
const BEATS: SliderSegment[] = [
  { from: 400, to: 2400, label: 'Evidence' },
  { from: 2400, to: 3700, label: 'Centre line' },
  { from: 3700, to: 5400, label: 'Unroll' },
  { from: 5400, to: 9000, label: 'Search' },
  { from: 9000, to: 10400, label: 'Lock in' },
  { from: 10400, to: 11600, label: 'Confidence' },
];
const beatAt = (time: number) => BEATS.find((beat) => time < beat.to)?.label ?? 'Finished';

const letGo: (() => void)[] = [];
afterEach(() => {
  for (const stop of letGo.splice(0)) stop();
});

function show(value: number, onChange: (value: number) => void = () => undefined, disabled = false) {
  putOnThePage(<Slider label="Position in the automatic pass" segments={BEATS} value={value} step={500} valueText={beatAt} disabled={disabled} onChange={onChange} />);
  return find('[role="slider"]');
}

describe('Slider', () => {
  it('is a slider to a screen reader, with its value said in words', () => {
    const slider = show(7400);
    expect(slider.getAttribute('aria-label')).toBe('Position in the automatic pass');
    expect(slider.getAttribute('aria-valuemin')).toBe('400');
    expect(slider.getAttribute('aria-valuemax')).toBe('11600');
    expect(slider.getAttribute('aria-valuenow')).toBe('7400');
    expect(slider.getAttribute('aria-valuetext')).toBe('Search');
    expect(slider.tabIndex).toBe(0);
  });

  it('draws one bar for each part, with the parts before the value done and the one it is in live', () => {
    const slider = show(7400);
    const parts = [...slider.querySelectorAll<HTMLElement>('.slider-segment')];
    expect(parts.map((part) => part.dataset.part)).toEqual(['done', 'done', 'done', 'live', 'ahead', 'ahead']);
    expect(parts.map((part) => part.textContent)).toEqual(['Evidence', 'Centre line', 'Unroll', 'Search', 'Lock in', 'Confidence']);
    // 7400 is 2000 of the 3600 ms of Search: five ninths.
    expect(Number(parts[3]?.style.getPropertyValue('--p'))).toBeCloseTo(5 / 9, 10);
    expect(parts[0]?.style.getPropertyValue('--p')).toBe('1');
    expect(parts[4]?.style.getPropertyValue('--p')).toBe('0');
    // Each part is as wide as its share of the range.
    expect(parts.map((part) => part.style.getPropertyValue('--span'))).toEqual(['2000', '1300', '1700', '3600', '1400', '1200']);
  });

  it('marks every part done at the end', () => {
    const slider = show(11600);
    expect([...slider.querySelectorAll<HTMLElement>('.slider-segment')].every((part) => part.dataset.part === 'done')).toBe(true);
    expect(slider.getAttribute('aria-valuetext')).toBe('Finished');
  });

  it('moves a step with the arrow keys and jumps with Page Up, Page Down, Home and End', () => {
    const changed = vi.fn();
    const slider = show(7400, changed);
    focusOn(slider);
    pressKey('ArrowRight');
    pressKey('ArrowLeft');
    pressKey('PageUp');
    pressKey('PageDown');
    pressKey('Home');
    pressKey('End');
    expect(changed.mock.calls.map(([value]) => value as number)).toEqual([7900, 6900, 9000, 5400, 400, 11600]);
  });

  it('keeps its keys to itself: an arrow on the slider never answers the choice on screen', () => {
    const answerLeft = vi.fn();
    letGo.push(takeKeys('choice', ['ArrowLeft', 'ArrowRight'], answerLeft));
    const slider = show(7400);
    focusOn(slider);
    expect(pressKey('ArrowLeft').defaultPrevented).toBe(true);
    expect(answerLeft).not.toHaveBeenCalled();
  });

  it('leaves every other key alone, so Space still pauses the pass', () => {
    const pause = vi.fn();
    letGo.push(takeKeys('pass', ['Space'], pause));
    const slider = show(7400);
    focusOn(slider);
    pressKey(' ');
    expect(pause).toHaveBeenCalledTimes(1);
  });

  it('says nothing when a key would not move it', () => {
    const changed = vi.fn();
    const slider = show(11600, changed);
    focusOn(slider);
    pressKey('ArrowRight');
    pressKey('End');
    expect(changed).not.toHaveBeenCalled();
  });

  it('goes where the pointer presses, and follows it while it is held', () => {
    const changed = vi.fn();
    const slider = show(7400, changed);
    // jsdom lays nothing out and has no pointer capture, so each bar is
    // given its place by hand: six bars 100 px wide, 10 px apart.
    slider.setPointerCapture = () => undefined;
    slider.querySelectorAll('.slider-bar').forEach((bar, at) => {
      bar.getBoundingClientRect = () => new DOMRect(at * 110, 0, 100, 3);
    });
    const pointer = (type: string, clientX: number) => {
      send(slider, new MouseEvent(type, { bubbles: true, clientX, button: 0 }));
    };
    pointer('pointermove', 50);
    expect(changed).not.toHaveBeenCalled();
    pointer('pointerdown', 50); // half way along Evidence: 400 + 1000
    pointer('pointermove', 380); // half way along Search (330 to 430): 5400 + 1800
    pointer('pointerup', 380);
    pointer('pointermove', 600);
    expect(changed.mock.calls.map(([value]) => value as number)).toEqual([1400, 7200]);
  });

  it('when disabled takes no key, no press and no focus', () => {
    const changed = vi.fn();
    const slider = show(7400, changed, true);
    expect(slider.getAttribute('aria-disabled')).toBe('true');
    expect(slider.tabIndex).toBe(-1);
    pressKey('ArrowRight', {}, slider);
    send(slider, new MouseEvent('pointerdown', { bubbles: true, clientX: 50, button: 0 }));
    expect(changed).not.toHaveBeenCalled();
  });
});
