// Every batch of a comparison, against the first placement.

import type { BenchResult } from '@/shapes';
import { compare_layouts } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('compare-layouts', {
  run: (input) => compare_layouts(input) as BenchResult,
});
