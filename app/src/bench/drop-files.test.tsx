import { afterEach, describe, expect, it, vi } from 'vitest';
import { watchForDroppedFiles } from './drop-files';

function drag(type: string, files: Blob[] | null, relatedTarget: EventTarget | null = null): Event {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, 'dataTransfer', { value: { types: files === null ? ['text/plain'] : ['Files'], files: files ?? [], dropEffect: 'none' } });
  Object.defineProperty(event, 'relatedTarget', { value: relatedTarget });
  return event;
}
let stop: (() => void) | undefined;
afterEach(() => { stop?.(); document.body.replaceChildren(); });

describe('files dropped on the window', () => {
  it('lights the places only for files, and does not leave between page parts', () => {
    const held = vi.fn();
    stop = watchForDroppedFiles({ held, dropped: vi.fn() });
    document.body.dispatchEvent(drag('dragenter', null));
    expect(held).not.toHaveBeenCalled();
    const over = drag('dragover', []);
    document.body.dispatchEvent(over);
    expect(over.defaultPrevented).toBe(true);
    expect(held).toHaveBeenLastCalledWith({ over: true, target: null });
    document.body.dispatchEvent(drag('dragleave', [], document.body));
    expect(held).toHaveBeenCalledTimes(1);
    document.body.dispatchEvent(drag('dragleave', []));
    expect(held).toHaveBeenLastCalledWith({ over: false, target: null });
  });
  it('uses the closest named place, including a tile and its children', () => {
    const held = vi.fn();
    const dropped = vi.fn();
    stop = watchForDroppedFiles({ held, dropped });
    const tile = document.body.appendChild(document.createElement('div'));
    tile.setAttribute('data-place', 'side');
    const child = tile.appendChild(document.createElement('span'));
    child.dispatchEvent(drag('dragover', []));
    expect(held).toHaveBeenLastCalledWith({ over: true, target: 'side' });
    const files = [new Blob(['made-up']), new Blob(['second'])];
    const event = drag('drop', files);
    child.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(dropped).toHaveBeenCalledWith(files, 'side');
    expect(held).toHaveBeenLastCalledWith({ over: false, target: null });
  });
  it('takes a drop anywhere with no named place, and removes all listeners', () => {
    const dropped = vi.fn();
    stop = watchForDroppedFiles({ held: vi.fn(), dropped });
    const files = [new Blob(['made-up'])];
    document.body.dispatchEvent(drag('drop', files));
    expect(dropped).toHaveBeenCalledWith(files, null);
    stop();
    document.body.dispatchEvent(drag('drop', files));
    document.body.dispatchEvent(drag('drop', null));
    expect(dropped).toHaveBeenCalledTimes(1);
  });
});
