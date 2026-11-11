import { describe, expect, it } from 'vitest';
import { pagesFrom } from './find-pages';
import type { GalleryPage } from './gallery-page';

const page = (area: string, part: string): GalleryPage => ({ area, part, states: [] });

describe('finding gallery pages', () => {
  it('gives each page its path under src/ as its address', () => {
    const found = pagesFrom({
      '../building-blocks/Button.gallery.tsx': page('building-blocks', 'Button'),
      '../building-blocks/toast/Toast.gallery.tsx': page('building-blocks', 'Toast'),
      '../steps/4-adjust/AdjustScreen.gallery.tsx': page('steps', 'Adjust'),
    });
    expect(found.map(({ id }) => id).sort()).toEqual(['building-blocks/Button', 'building-blocks/toast/Toast', 'steps/4-adjust/AdjustScreen']);
  });

  it('lists pages by area, then by part, whatever order the files were found in', () => {
    const found = pagesFrom({
      '../marks/Pin.gallery.tsx': page('marks', 'Pin'),
      '../building-blocks/toast/Toast.gallery.tsx': page('building-blocks', 'Toast'),
      '../bench/Bay.gallery.tsx': page('bench', 'Bay'),
      '../building-blocks/Button.gallery.tsx': page('building-blocks', 'Button'),
    });
    expect(found.map(({ page: { area, part } }) => `${area}: ${part}`)).toEqual(['bench: Bay', 'building-blocks: Button', 'building-blocks: Toast', 'marks: Pin']);
  });

  it('keeps two pages that give the same area and part, each under its own address', () => {
    const found = pagesFrom({
      '../marks/b/Pin.gallery.tsx': page('marks', 'Pin'),
      '../marks/a/Pin.gallery.tsx': page('marks', 'Pin'),
    });
    expect(found.map(({ id }) => id)).toEqual(['marks/a/Pin', 'marks/b/Pin']);
  });

  it('has a page for every building block', () => {
    // The files as they are on disk; none is loaded.
    const files = Object.keys(import.meta.glob('../building-blocks/**/*.gallery.tsx'));
    for (const block of ['Button', 'IconButton', 'Kbd', 'Tooltip', 'Popover', 'Dialog', 'Slider', 'toast/Toast', 'keyboard/keyboard']) {
      expect(files).toContain(`../building-blocks/${block}.gallery.tsx`);
    }
  });
});
