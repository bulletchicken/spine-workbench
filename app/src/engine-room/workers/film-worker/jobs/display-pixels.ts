// Fresh binary16 values in [0, 1]; window and level belong to the viewer.

import { defineJob } from '../../boot';

export default defineJob('display-pixels', {
  run: (_input, room) => room.desk().display_pixels(),
});
