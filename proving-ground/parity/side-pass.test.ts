// A drawn greyscale ramp exercises the optional side export in either build.
import { readFileSync } from 'node:fs';
import { beforeAll, expect, it } from 'vitest';
import { FilmDesk, engine_info, initSync } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import type { EngineInfo, Film, FilmFacts, Step, TrainedPassResult } from '../../app/src/shapes';

const png = Uint8Array.from(Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAGAAAACACAAAAADRtzNmAAABl0lEQVR4nO2YyXKEMBBDQ6n1/9+bHJJU5UBYbPAiHYzp25i23gzFUO63fMBbC82AcADoBXD3SQ8grAAeF7QAnpekAMIKYHJVBmBmXQUgrADmLykAvLooABBWAG+udwJ429EHIKwAljS1A1jW1gwgrAAWdzYBWNHbAiCsANa11wJY114NIKwAVsdXAdgQXwMgrAC2xZcC2BpfCCCsAHbEFwDYFX8PIKwA9sZfAyRnssinaA6VkYtRHYojHaQ71EcySjiVRCJLOlXFKU08FcYxTz3Wxj5QP5bHNtKhFWITavEiQWs8V4DH6nD9BaavD2D5greWHzcgzIAYUKlhF/kqtUOdb/ir1P4r8zi+Su2vrv6s0ys13L7KZlZqKNs5qVJDxab5lBpq+6dSamh64GZRamh/Y02g1NB5/H62UoNiOHmsUoNsdIsHKjVoB9t4llKDYeyPwZUauAI4rFL7hLeWbzcAIyo1bEJfpXaq4y0X3yIOrdSQjNIBOLRSQz5FAuDQSg03AXMrNZTsnVapoXjbjEoNdTvGV2q/VgqXub5ji0QAAAAASUVORK5CYII=", 'base64'));
beforeAll(() => { initSync({ module: readFileSync(new URL('../../app/src/engine-room/wasm-build/wasm_bridge_bg.wasm', import.meta.url)) }); });

it('reports availability and either refuses plainly or produces five reviewable lumbar candidates', () => {
  const available = (engine_info() as EngineInfo).trainedSideFinder;
  const desk = FilmDesk.open(png, 'side');
  try {
    const facts = desk.facts() as FilmFacts;
    const film: Film = {
      kind: 'side', image: { path: 'films/side.png', width: facts.width, height: facts.height, bitsStored: facts.bitsStored },
      source: facts.source, keptTags: facts.keptTags, values: facts.values, flags: facts.flags, blanked: [],
      geometry: { mmPerPx: null, mmPerPxVertical: null, scale: null, quarterTurns: 0, imageRight: 'posterior',
        confirmed: { upright: false, leftRight: false, facing: true, scale: false, polarity: false, privacy: false } },
    };
    const unconfirmed = { ...film, geometry: { ...film.geometry, confirmed: { ...film.geometry.confirmed, facing: false } } };
    expect(() => desk.start_trained_side_pass(unconfirmed, 1)).toThrow(expect.objectContaining({ code: available ? 'pass-facing-required' : 'side-finder-unavailable' }));
    if (!available) {
      expect(() => desk.start_trained_side_pass(film, 1)).toThrow(expect.objectContaining({ code: 'side-finder-unavailable' }));
      return;
    }
    const pass = desk.start_trained_side_pass(film, 1);
    let consumed = false;
    try {
      expect(pass.advance(0)).toMatchObject({ done: false, frames: [] });
      for (let i = 0; ; i++) {
        const step = pass.advance([1, 7, 2, 31][i % 4] ?? 1) as Step;
        if (step.done) break;
      }
      consumed = true;
      const result = pass.finish() as TrainedPassResult;
      expect(result.marks.vertebrae.map((v) => [v.id, v.level])).toEqual([[1, 'L1'], [2, 'L2'], [3, 'L3'], [4, 'L4'], [5, 'L5']]);
      expect(result.marks.naming.flags).toEqual(['estimated']);
      expect(result.warnings).toEqual([]);
      for (const v of result.marks.vertebrae) expect(v.confidence).toMatchObject({ score: 0, band: 'low', calibrated: false, reasons: ['level-uncertain'] });
    } finally { if (!consumed) pass.free(); }
  } finally { desk.free(); }
}, 30_000);
