// A film worker: one per loaded film, holding that film's FilmDesk.

import { ownScope, startWorker } from '../boot';

startWorker(ownScope(), 'film');
