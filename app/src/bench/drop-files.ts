import type { FilmKind } from '@/shapes';
import type { Held } from './Stage';

function carriesFiles(event: DragEvent): boolean {
  return event.dataTransfer?.types.includes('Files') ?? false;
}

function placeUnder(event: DragEvent): FilmKind | null {
  const kind = event.target instanceof Element ? event.target.closest('[data-place]')?.getAttribute('data-place') : null;
  return kind === 'front' || kind === 'side' || kind === 'bend-left' || kind === 'bend-right' ? kind : null;
}

/** Window listeners keep a dropped file from replacing the page, wherever it lands. */
export function watchForDroppedFiles(hear: { held(held: Held): void; dropped(files: Blob[], onPlace: FilmKind | null): void }): () => void {
  const onOver = (event: DragEvent) => {
    if (!carriesFiles(event)) return;
    event.preventDefault();
    if (event.dataTransfer !== null) event.dataTransfer.dropEffect = 'copy';
    hear.held({ over: true, target: placeUnder(event) });
  };
  const onLeave = (event: DragEvent) => {
    if (event.relatedTarget === null) hear.held({ over: false, target: null });
  };
  const onDrop = (event: DragEvent) => {
    if (!carriesFiles(event)) return;
    event.preventDefault();
    hear.held({ over: false, target: null });
    hear.dropped(Array.from(event.dataTransfer?.files ?? []), placeUnder(event));
  };
  window.addEventListener('dragenter', onOver);
  window.addEventListener('dragover', onOver);
  window.addEventListener('dragleave', onLeave);
  window.addEventListener('drop', onDrop);
  return () => {
    window.removeEventListener('dragenter', onOver);
    window.removeEventListener('dragover', onOver);
    window.removeEventListener('dragleave', onLeave);
    window.removeEventListener('drop', onDrop);
  };
}
