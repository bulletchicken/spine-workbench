// A made-up spine from a recipe: its four films as PNG bytes, and their
// answer key. The films open through the same path as any dropped film.

import type { MadeUpSpine } from '@/shapes';
import { make_up_spine } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('make-up-spine', {
  run: (recipe) => make_up_spine(recipe) as MadeUpSpine,
});
