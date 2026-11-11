// A scale proposal from two points on a ruler of known length.

import type { ScaleResult } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('scale-from-ruler', {
  run: ({ a, b, mm }, room) => room.desk().scale_from_ruler(a, b, mm) as ScaleResult,
});
