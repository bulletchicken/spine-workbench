import { describe, expect, it } from 'vitest';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { filmRecordFrom } from './film-record';

describe('the record of a newly read film', () => {
  it('keeps the facts and no geometry proposal or confirmation', () => {
    const facts = { ...smallStory().films.side.facts, exifQuarterTurns: 1 as const };
    const film = filmRecordFrom(facts);
    expect(film).toEqual({
      kind: 'side', image: { path: 'films/side.png', width: facts.width, height: facts.height, bitsStored: facts.bitsStored },
      source: facts.source, keptTags: facts.keptTags, values: facts.values, flags: facts.flags, blanked: [],
      geometry: { mmPerPx: null, mmPerPxVertical: null, scale: null, quarterTurns: 0, imageRight: null,
        confirmed: { upright: false, leftRight: false, facing: false, scale: false, polarity: false, privacy: false } },
    });
    expect(film).not.toHaveProperty('exifQuarterTurns');
    expect(film).not.toHaveProperty('name');
  });
});
