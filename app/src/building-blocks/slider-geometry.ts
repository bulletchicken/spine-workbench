// The slider's arithmetic, kept apart from its drawing so it can be
// checked with numbers worked out by hand.

/** One part of a slider's range, drawn as its own bar. Parts are in order and share their ends. */
export interface SliderSegment {
  from: number;
  to: number;
  /** The caption under its bar. */
  label?: string;
}

/** A bar's left and right edges on screen. */
export interface BarEdges {
  left: number;
  right: number;
}

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

/** How much of a segment lies at or before `value`, from 0 to 1. */
export function filledPart(segment: SliderSegment, value: number): number {
  if (segment.to <= segment.from) return value >= segment.to ? 1 : 0;
  return clamp((value - segment.from) / (segment.to - segment.from), 0, 1);
}

/** The segment `value` is in. A value on a shared end belongs to the later segment; the range's end to the last. */
export function segmentAt(segments: readonly SliderSegment[], value: number): number {
  const later = segments.findIndex((segment) => value < segment.to);
  return later === -1 ? segments.length - 1 : later;
}

/**
 * The value under a pointer at screen position `x`, given where each
 * segment's bar is drawn. Inside a bar it is that bar's share of its
 * segment, so the fill's edge lands exactly under the pointer; in the gap
 * between two bars it is the end they share; past either end of the
 * slider it is that end.
 */
export function valueAtPointer(x: number, bars: readonly BarEdges[], segments: readonly SliderSegment[]): number {
  for (const [at, segment] of segments.entries()) {
    const bar = bars[at];
    if (bar === undefined) break;
    if (x > bar.right && at < segments.length - 1) continue;
    const width = bar.right - bar.left;
    const share = width > 0 ? clamp((x - bar.left) / width, 0, 1) : 0;
    return segment.from + share * (segment.to - segment.from);
  }
  return segments[0]?.from ?? 0;
}

/**
 * Where a key takes the slider from `value`. Arrows move one `step`;
 * Page Up and Page Down go to the next and the previous segment's start;
 * Home and End go to the ends. `null` for any other key.
 */
export function valueAfterKey(key: string, value: number, step: number, segments: readonly SliderSegment[]): number | null {
  const min = segments[0]?.from ?? 0;
  const max = segments.at(-1)?.to ?? 0;
  switch (key) {
    case 'ArrowRight':
    case 'ArrowUp':
      return clamp(value + step, min, max);
    case 'ArrowLeft':
    case 'ArrowDown':
      return clamp(value - step, min, max);
    case 'PageUp':
      return segments.find((segment) => segment.from > value)?.from ?? max;
    case 'PageDown':
      return segments.findLast((segment) => segment.from < value)?.from ?? min;
    case 'Home':
      return min;
    case 'End':
      return max;
    default:
      return null;
  }
}
