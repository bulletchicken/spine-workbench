// A whole placement after its ends, anchors or rods are changed by hand.

import type { ImplantLayout } from '@/shapes';
import { rebuild_layout } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('rebuild-layout', {
  run: (input) => rebuild_layout(input) as ImplantLayout,
});
