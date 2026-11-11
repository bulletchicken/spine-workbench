// A circle fit from a centre-to-rim drag and the object's known diameter.

import type { ScaleResult } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('scale-from-known-object', {
  run: ({ near, edge, diameterMm }, room) =>
    room.desk().scale_from_known_object(near, edge, diameterMm) as ScaleResult,
});
