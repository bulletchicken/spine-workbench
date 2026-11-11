import { useState, type CSSProperties, type KeyboardEvent, type PointerEvent } from 'react';
import './Slider.css';
import { filledPart, segmentAt, valueAfterKey, valueAtPointer, type SliderSegment } from './slider-geometry';

interface SliderProps {
  /** Its spoken name. */
  label: string;
  /** The range, as one or more parts in order. Each part is its own bar, as wide as its share of the range, with its caption under it. */
  segments: readonly SliderSegment[];
  value: number;
  /** How far one arrow key moves it. */
  step: number;
  /** The value in words, for a screen reader: the part it is in, not a bare number. */
  valueText: (value: number) => string;
  disabled?: boolean;
  onChange: (value: number) => void;
  className?: string;
}

/**
 * A slider whose track is cut into named parts, such as the six beats of
 * the pass. Press or drag anywhere on it; with the keyboard, arrows move
 * a step, Page Up and Page Down jump a part, Home and End go to the ends.
 * The filled length is the value; a knob shows at its end while the
 * slider is under the pointer, focused or dragged.
 */
export function Slider({ label, segments, value, step, valueText, disabled = false, onChange, className }: SliderProps) {
  const [dragging, setDragging] = useState(false);
  const min = segments[0]?.from ?? 0;
  const max = segments.at(-1)?.to ?? 0;
  const held = Math.min(max, Math.max(min, value));
  const knobIn = segmentAt(segments, held);

  const valueUnder = (event: PointerEvent<HTMLDivElement>) => {
    const bars = [...event.currentTarget.querySelectorAll('.slider-bar')].map((bar) => bar.getBoundingClientRect());
    return valueAtPointer(event.clientX, bars, segments);
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (disabled || event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDragging(true);
    onChange(valueUnder(event));
  };
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (dragging) onChange(valueUnder(event));
  };
  const letGo = () => {
    setDragging(false);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (disabled || event.metaKey || event.ctrlKey || event.altKey) return;
    const next = valueAfterKey(event.key, held, step, segments);
    if (next === null) return;
    // Its own keys: handled here, and never walked down the ranks.
    event.preventDefault();
    if (next !== held) onChange(next);
  };

  return (
    <div
      role="slider"
      tabIndex={disabled ? -1 : 0}
      aria-label={label}
      aria-orientation="horizontal"
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={held}
      aria-valuetext={valueText(held)}
      aria-disabled={disabled || undefined}
      className={className === undefined ? 'slider' : `slider ${className}`}
      data-dragging={dragging || undefined}
      data-captions={segments.some((segment) => segment.label !== undefined) || undefined}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={letGo}
      onPointerCancel={letGo}
      onKeyDown={onKeyDown}
    >
      {segments.map((segment, at) => (
        <div
          key={`${String(segment.from)} ${String(segment.to)}`}
          className="slider-segment"
          data-part={held >= segment.to ? 'done' : held >= segment.from ? 'live' : 'ahead'}
          style={{ '--span': segment.to - segment.from, '--p': filledPart(segment, held) } as CSSProperties}
        >
          <div className="slider-bar">
            <div className="slider-rail">
              <i className="slider-fill" />
            </div>
            {at === knobIn && <i className="slider-knob" />}
          </div>
          {segment.label !== undefined && <span className="slider-caption">{segment.label}</span>}
        </div>
      ))}
    </div>
  );
}
