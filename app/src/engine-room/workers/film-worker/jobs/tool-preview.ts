// Hover previews are replaceable through the job table's queue rule.

import type { ToolPreview } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('tool-preview', {
  run: (input, room) => room.desk().tool_preview(input) as ToolPreview,
});
