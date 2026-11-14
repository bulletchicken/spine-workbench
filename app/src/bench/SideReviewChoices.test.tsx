import { expect, it, vi } from 'vitest';
import { find, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { SideReview } from '@/review/SideReview';

it('offers the names-one-too-low choice and sends its rename action', () => {
  const onRename = vi.fn();
  putOnThePage(<SideReview onLooksRight={() => undefined} onRename={onRename} onLeaveOut={() => undefined} />);
  const button = find('button:nth-of-type(3)');
  expect(button.textContent).toBe('The names are one too low');
  expect(button.hasAttribute('disabled')).toBe(false);
  pressOn(button);
  expect(onRename).toHaveBeenCalledExactlyOnceWith('too-low');
});
