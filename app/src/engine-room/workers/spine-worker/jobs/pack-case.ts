// One case archive, checked strictly before any bytes leave the engine.

import { pack_case } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('pack-case', {
  run: (opened) => pack_case(opened),
});
