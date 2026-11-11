// The engine's proposals for one film: scale, upright, facing and markers.

import type { GeometryGuess } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('sense-geometry', {
  run: (_input, room) => room.desk().sense_geometry() as GeometryGuess,
});
