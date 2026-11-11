import { describe, expect, it } from 'vitest';
import { searchFor, viewFrom, type GalleryView } from './gallery-view';

describe('the gallery’s address', () => {
  it('reads a bare ?gallery as the first page, 1440 wide, full motion, with the index', () => {
    expect(viewFrom('?gallery')).toEqual({ page: null, width: 1440, motion: 'full', alone: false });
    expect(viewFrom('?gallery=')).toEqual({ page: null, width: 1440, motion: 'full', alone: false });
  });

  it('reads the page, the width, the motion and whether it stands alone', () => {
    expect(viewFrom('?gallery=building-blocks/Button&width=1024&motion=reduced&alone')).toEqual({
      page: 'building-blocks/Button',
      width: 1024,
      motion: 'reduced',
      alone: true,
    });
  });

  it('falls back to 1440 and full motion for anything it does not know', () => {
    expect(viewFrom('?gallery=marks/Pin&width=800&motion=slow')).toEqual({ page: 'marks/Pin', width: 1440, motion: 'full', alone: false });
  });

  it('writes the shortest address that says the same', () => {
    expect(searchFor({ page: null, width: 1440, motion: 'full', alone: false })).toBe('?gallery');
    expect(searchFor({ page: 'building-blocks/toast/Toast', width: 1440, motion: 'full', alone: true })).toBe('?gallery=building-blocks/toast/Toast&alone');
    expect(searchFor({ page: 'marks/Pin', width: 1024, motion: 'reduced', alone: false })).toBe('?gallery=marks/Pin&width=1024&motion=reduced');
  });

  it('reads back every address it writes', () => {
    const views: GalleryView[] = [];
    for (const page of [null, 'building-blocks/Button', 'steps/4-adjust/AdjustScreen', 'marks/a page with spaces'])
      for (const width of [1440, 1024] as const)
        for (const motion of ['full', 'reduced'] as const) for (const alone of [false, true]) views.push({ page, width, motion, alone });
    for (const view of views) expect(viewFrom(searchFor(view))).toEqual(view);
  });
});
