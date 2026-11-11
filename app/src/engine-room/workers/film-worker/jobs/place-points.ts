// Turn the solved chain into marks on the film held by this worker.

import type { FilmMarks } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('place-points', {
  run: ({ chain, firstVertebraId }, room) => room.desk().place_points(chain, firstVertebraId) as FilmMarks,
});
