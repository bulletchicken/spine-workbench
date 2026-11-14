import { useLayoutEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import './StageCaption.css';

/** The caption follows the real dock height, including an open review card. */
export function StageCaption({ children, className = '', reserveRoom = false }: { children: ReactNode; className?: string; reserveRoom?: boolean }) {
  const caption = useRef<HTMLParagraphElement>(null);
  useLayoutEffect(() => {
    const part = caption.current, surface = part?.parentElement;
    const frame = part?.closest('.frame, .place-gallery'), dock = frame?.querySelector('.dock');
    if (part == null || surface == null || dock == null) return;
    const place = () => {
      const gap = parseFloat(getComputedStyle(part).getPropertyValue('--space-12'));
      if (reserveRoom) surface.style.setProperty('--stage-caption-room', `${String(part.offsetHeight + gap)}px`);
      part.style.top = `${String(dock.getBoundingClientRect().top - surface.getBoundingClientRect().top - gap - part.offsetHeight)}px`;
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(dock); observer.observe(surface); observer.observe(part);
    window.addEventListener('resize', place);
    return () => { observer.disconnect(); window.removeEventListener('resize', place); if (reserveRoom) surface.style.removeProperty('--stage-caption-room'); };
  }, [reserveRoom]);
  return <p ref={caption} className={`stage-caption ${className}`}>{children}</p>;
}
