// Placement proposals on a fitted spine, counted over versions of it.

import type { LayoutProposals } from '@/shapes';
import { propose_layouts } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('propose-layouts', {
  run: (input) => propose_layouts(input) as LayoutProposals,
});
