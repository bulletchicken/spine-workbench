// Findings for the case's current films, marks, naming and confirmations.

import type { CheckFinding } from '@/shapes';
import { check_case } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('check-case', {
  run: (record) => check_case(record) as CheckFinding[],
});
