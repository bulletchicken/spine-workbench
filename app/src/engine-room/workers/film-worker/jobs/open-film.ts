// Open one film completely before replacing the desk this worker holds.

import type { FilmFacts } from '@/shapes';
import { EngineFailure } from '../../../engine-client';
import { FilmDesk } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('open-film', {
  run: ({ kind, bytes, saved }, room) => {
    if (saved !== null && saved.kind !== kind) {
      throw new EngineFailure({ code: 'engine-out-of-sync', message: 'the saved film belongs to another slot', detail: null });
    }
    const pixels = new Uint8Array(bytes);
    const desk = saved === null ? FilmDesk.open(pixels, kind) : FilmDesk.open_saved(pixels, saved);
    try {
      const facts = desk.facts() as FilmFacts;
      const thumbnail = desk.thumbnail(256);
      room.holdDesk(desk);
      return { facts, thumbnail };
    } catch (error) {
      desk.free();
      throw error;
    }
  },
});
