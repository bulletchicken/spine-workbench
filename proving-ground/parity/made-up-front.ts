import { make_up_spine } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import type { MadeUpRecipe, MadeUpSpine } from '../../app/src/shapes';

/** The committed trained-pass recipe, with no answer-key geometry read. */
export const TRAINED_FRONT_RECIPE: MadeUpRecipe = { name: 'straight', seed: 7, tier: 't2' };

export function madeUpFront(): Uint8Array {
  const film = (make_up_spine(TRAINED_FRONT_RECIPE) as MadeUpSpine).films.find((film) => film.kind === 'front');
  if (film === undefined) throw new Error('the made-up recipe has no Front PNG');
  return film.png;
}
