// The engine keeps pins fixed while the affected marks re-solve.

import type { PinOutput } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('resolve-pins', {
  run: (input, room) => room.desk().resolve_pins(input) as PinOutput,
});
