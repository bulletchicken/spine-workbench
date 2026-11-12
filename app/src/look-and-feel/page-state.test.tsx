// The state of the page reaches the design system's stylesheet as
// attributes on <html>. Each case reads <html> as the stylesheet would.

import { describe, expect, it } from 'vitest';
import { putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { usePageState, type PageStateName } from './page-state';

function Says({ name, value }: { name: PageStateName; value: string | boolean | null }) {
  usePageState(name, value);
  return null;
}

const page = document.documentElement;

describe('usePageState', () => {
  it('writes a string as the attribute’s value', () => {
    putOnThePage(<Says name="moment" value="find" />);
    expect(page.getAttribute('data-moment')).toBe('find');
  });

  it('writes true as an empty attribute, and false or null as none', () => {
    const shown = putOnThePage(<Says name="drag" value={true} />);
    expect(page.getAttribute('data-drag')).toBe('');
    shown.show(<Says name="drag" value={false} />);
    expect(page.hasAttribute('data-drag')).toBe(false);
    shown.show(<Says name="drag" value="" />);
    expect(page.getAttribute('data-drag')).toBe('');
    shown.show(<Says name="drag" value={null} />);
    expect(page.hasAttribute('data-drag')).toBe(false);
  });

  it('follows a change of value', () => {
    const shown = putOnThePage(<Says name="films" value="empty" />);
    shown.show(<Says name="films" value="ready" />);
    expect(page.getAttribute('data-films')).toBe('ready');
  });

  it('takes the attribute away when its writer leaves the page', () => {
    const shown = putOnThePage(<Says name="leaving" value={true} />);
    expect(page.hasAttribute('data-leaving')).toBe(true);
    shown.takeDown();
    expect(page.hasAttribute('data-leaving')).toBe(false);
  });
});
