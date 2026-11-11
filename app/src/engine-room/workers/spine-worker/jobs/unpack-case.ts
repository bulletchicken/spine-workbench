// A case archive read completely before its record and films are returned.

import type { OpenedCase } from '@/shapes';
import { unpack_case } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('unpack-case', {
  run: ({ bytes }) => unpack_case(new Uint8Array(bytes)) as OpenedCase,
});
