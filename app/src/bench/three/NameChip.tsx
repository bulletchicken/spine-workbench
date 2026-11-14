import { useLayoutEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import type { Level } from '@/shapes';
import './name-chip.css';
export function NameChip({ level, x, y }: { level: Level | null; x: number; y: number }) {
  const chip = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const part = chip.current;
    if (part === null) return;
    const css = getComputedStyle(part);
    const dx = parseFloat(css.getPropertyValue('--chip-dx')) || 0, dy = parseFloat(css.getPropertyValue('--chip-dy')) || 0;
    part.style.left = `${String(Math.max(0, Math.min(window.innerWidth - part.offsetWidth, x + dx)))}px`;
    part.style.top = `${String(Math.max(0, Math.min(window.innerHeight - part.offsetHeight, y + dy)))}px`;
  }, [level, x, y]);
  return createPortal(<div ref={chip} className={`chip glass num${level === null ? '' : ' show'}`} aria-hidden="true">{level}</div>, document.body);
}
