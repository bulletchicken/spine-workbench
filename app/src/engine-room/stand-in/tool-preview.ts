// What a stand-in tool would do at the point the pointer is over, by the
// same rules the boundary and region wands act on.

import type { ToolPreview, ToolPreviewInput, VertebraOnFilm } from '@/shapes';
import { boundaryHit, levelsAfterMerge, levelsAfterSplit } from './boundary-wand';
import { lineAcross, quadOf } from './quads';
import { regionHit } from './region-wand';
import type { AdjustScript } from './scripts/adjust';
import type { ToolFilm } from './tool-film';

export function previewTool(film: ToolFilm, input: ToolPreviewInput, script: AdjustScript): ToolPreview {
  const { caseVersion, marks, at } = input;
  if (input.tool === 'boundary-wand') {
    const hit = boundaryHit(film, marks, at, script);
    switch (hit.kind) {
      case 'none':
        return { action: 'none', caseVersion, reason: hit.reason };
      case 'split': {
        const vertebra = marks.vertebrae[hit.index] as VertebraOnFilm;
        return {
          action: 'split',
          caseVersion,
          vertebra: vertebra.id,
          line: lineAcross(quadOf(vertebra), hit.down),
          levelsAfter: levelsAfterSplit(marks.vertebrae, hit, input.newVertebraId),
        };
      }
      case 'merge': {
        const upper = marks.vertebrae[hit.upper] as VertebraOnFilm;
        const lower = marks.vertebrae[hit.upper + 1] as VertebraOnFilm;
        return { action: 'merge', caseVersion, upper: upper.id, lower: lower.id, levelsAfter: levelsAfterMerge(marks.vertebrae, hit) };
      }
    }
  }
  const hit = regionHit(film, marks, at, script);
  if (hit.kind === 'none') return { action: 'none', caseVersion, reason: hit.reason };
  return { action: hit.kind, caseVersion, vertebra: hit.vertebra.id };
}
