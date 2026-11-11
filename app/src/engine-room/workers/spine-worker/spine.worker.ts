// The spine worker: exactly one. It runs the chain pass, which needs every
// film, and the case checks, case files, made-up spines and answer keys.
// Its jobs are the files in ./jobs, one per job, each named after its job.

import { ownScope, startWorker } from '../boot';

startWorker(ownScope(), 'spine', import.meta.glob(['./jobs/*.ts', '!./jobs/*.test.ts'], { eager: true }));
