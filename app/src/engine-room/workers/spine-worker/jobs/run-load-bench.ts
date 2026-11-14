// Every placement on one slice of versions, through every stop.

import type { BenchBatch } from '@/shapes';
import { run_load_bench } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('run-load-bench', {
  run: (input) => run_load_bench(input) as BenchBatch,
});
