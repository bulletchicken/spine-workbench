import type { Spot } from './stage-layout';

interface Flight { x: number; y: number; s: number }

/** Scaling about the new box's centre makes its edges meet the old box's. */
export function flightFrom(was: Spot, now: Spot): Flight {
  return { x: was.x + (was.width - now.width) / 2, y: was.y + (was.height - now.height) / 2, s: was.width / now.width };
}

const cleanups = new WeakMap<HTMLElement, () => void>();

/** Reading the layout box commits the still pose before transitions are restored. */
export function readLayout(part: HTMLElement): number {
  return part.offsetWidth;
}

/** The caller has written the new real boxes before their inverse transforms are applied. */
export function fly(tiles: HTMLElement, starts: ReadonlyMap<HTMLElement, Flight>, rests: ReadonlyMap<HTMLElement, { x: number; y: number }>): void {
  cleanups.get(tiles)?.();
  tiles.classList.add('still');
  const pending = new Set<HTMLElement>();
  for (const [tile, start] of starts) {
    const rest = rests.get(tile);
    if (rest === undefined || (start.x === rest.x && start.y === rest.y && start.s === 1)) continue;
    pending.add(tile);
    tile.style.setProperty('--x', String(start.x));
    tile.style.setProperty('--y', String(start.y));
    tile.style.setProperty('--s', String(start.s));
  }
  readLayout(tiles);
  tiles.classList.remove('still');
  if (pending.size > 0) {
    tiles.setAttribute('data-flying', '');
    // Outlines opt into the pointer themselves, so inheritance alone cannot exclude them.
    tiles.inert = true;
  }
  for (const [tile, rest] of rests) {
    tile.style.setProperty('--x', String(rest.x));
    tile.style.setProperty('--y', String(rest.y));
    tile.style.setProperty('--s', '1');
  }
  const done = () => {
    tiles.removeAttribute('data-flying');
    tiles.inert = false;
    tiles.removeEventListener('transitionend', ended);
    tiles.removeEventListener('transitioncancel', ended);
    cleanups.delete(tiles);
  };
  const ended = (event: TransitionEvent) => {
    if (event.propertyName !== 'transform' || !(event.target instanceof HTMLElement)) return;
    pending.delete(event.target);
    if (pending.size === 0) done();
  };
  if (pending.size === 0) return;
  tiles.addEventListener('transitionend', ended);
  tiles.addEventListener('transitioncancel', ended);
  cleanups.set(tiles, done);
}
