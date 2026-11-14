import { useLayoutEffect, useRef } from 'react';
import { PLACE_WORDS } from '@/words/place';

/** The caption follows the real dock height, including an open review card. */
export function PlacementCaption() {
  const caption = useRef<HTMLParagraphElement>(null);
  useLayoutEffect(() => {
    const part = caption.current, surface = part?.parentElement;
    const frame = part?.closest('.frame, .place-gallery'), dock = frame?.querySelector('.dock');
    if (part == null || surface == null || dock == null) return;
    const place = () => {
      const gap = parseFloat(getComputedStyle(part).getPropertyValue('--space-12'));
      part.style.top = `${String(dock.getBoundingClientRect().top - surface.getBoundingClientRect().top - gap - part.offsetHeight)}px`;
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(dock); observer.observe(surface); observer.observe(part);
    window.addEventListener('resize', place);
    return () => { observer.disconnect(); window.removeEventListener('resize', place); };
  }, []);
  return <p ref={caption} className="place-caption">{PLACE_WORDS.sizing}</p>;
}
