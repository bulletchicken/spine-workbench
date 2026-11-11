// Run the film stages through the worker's shared step loop.

import type { FilmPassResult, Step } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('film-pass', {
  start: ({ film }, room) => {
    const pass = room.desk().start_film_pass(film);
    return {
      advance: (workUnits) => pass.advance(workUnits) as Step,
      finish: () => pass.finish() as FilmPassResult,
      free: () => {
        pass.free();
      },
    };
  },
});
