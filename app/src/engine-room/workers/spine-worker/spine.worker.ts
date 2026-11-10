// The spine worker: exactly one. It runs the chain pass, which needs every
// film, and the case checks, case files, made-up spines and answer keys.

import { ownScope, startWorker } from '../boot';

startWorker(ownScope(), 'spine');
