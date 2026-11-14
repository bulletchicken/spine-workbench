import type { Level } from '@/shapes';
import type { MarkToDraw } from '@/marks/mark-to-draw';
/** Each film owns its ids; the level is what survives across films. */
export function markAtLevel(marks: readonly MarkToDraw[], level: Level | null): string | null {
  return level === null ? null : marks.find((mark) => mark.name === level)?.key ?? null;
}
