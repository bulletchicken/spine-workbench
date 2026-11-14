import { expect, it } from 'vitest';
import type { MarkToDraw } from '@/marks/mark-to-draw';
import { markAtLevel } from './spine-links';
it('links by level across films with different ids and leaves absent marks alone', () => {
  const marks = (key: string) => [{ key, name: 'T8' } as MarkToDraw];
  expect(markAtLevel(marks('front-3'), 'T8')).toBe('front-3');
  expect(markAtLevel(marks('side-19'), 'T8')).toBe('side-19');
  expect(markAtLevel(marks('side-19'), 'T9')).toBeNull();
});
