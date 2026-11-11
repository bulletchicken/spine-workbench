// Placed marks compared with the made-up spine's visible true corners.

import type { AnswerKeyComparison } from '@/shapes';
import { compare_with_answer_key } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('compare-with-answer-key', {
  run: ({ marks, key }) => compare_with_answer_key(marks, key) as AnswerKeyComparison,
});
