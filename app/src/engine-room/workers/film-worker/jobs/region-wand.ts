// Growth tiles travel in the answer; the screen sets their animation pace.

import type { RegionWandOutput } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('region-wand', {
  run: (input, room) => room.desk().region_wand(input) as RegionWandOutput,
});
