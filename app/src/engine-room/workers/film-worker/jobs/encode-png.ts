// The stored film: current brightness and blanked boxes, at its own size.

import { defineJob } from '../../boot';

export default defineJob('encode-png', {
  run: ({ film }, room) => room.desk().encode_png(film),
});
