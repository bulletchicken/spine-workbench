// A film worker: one per loaded film, holding that film's FilmDesk. Its
// jobs are the files in ./jobs, one per job, each named after its job.

import { ownScope, startWorker } from '../boot';

startWorker(ownScope(), 'film', import.meta.glob(['./jobs/*.ts', '!./jobs/*.test.ts'], { eager: true }));
