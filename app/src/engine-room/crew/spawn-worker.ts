// Starts a real engine worker. The `new Worker(new URL(…))` form is what
// Vite recognises, so each worker is built as its own file beside the
// page and served from the page's origin.

import type { WorkerRole } from '../messages';
import type { SpawnWorker } from './crew-member';

export const spawnWorker: SpawnWorker = (role: WorkerRole, events) => {
  const worker =
    role === 'film'
      ? new Worker(new URL('../workers/film-worker/film.worker.ts', import.meta.url), { type: 'module' })
      : new Worker(new URL('../workers/spine-worker/spine.worker.ts', import.meta.url), { type: 'module' });
  worker.addEventListener('message', (event: MessageEvent) => {
    events.message(event.data as Parameters<typeof events.message>[0]);
  });
  worker.addEventListener('error', (event) => {
    event.preventDefault();
    events.failed(event.message);
  });
  worker.addEventListener('messageerror', () => {
    events.failed('a message from the worker could not be read');
  });
  return worker;
};
