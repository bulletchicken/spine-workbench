// Measurements on a fitted spine, with bands over its versions.

import type { SpineReadings } from '@/shapes';
import { measure_spine } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('measure-spine', {
  run: (input) => measure_spine(input) as SpineReadings,
});
