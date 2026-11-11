import type { StandInStory } from '@/sample-case/story-shape';
import { describe, expect, it } from 'vitest';
import { pickEngine } from './pick-engine';
import { smallStory } from './stand-in/small-story';

/** The story the stand-in should be telling: the sample case's once it exists, else its own. */
async function expectedStory(): Promise<StandInStory> {
  const sample = Object.values(import.meta.glob<StandInStory>('../sample-case/story.ts', { import: 'default' }))[0];
  return sample === undefined ? smallStory() : sample();
}

describe('picking the engine in development', () => {
  it.each(['', '?slow', '?engine=stand-in'])('hands the stand-in over for %j', async (address) => {
    const engine = await pickEngine(address);
    try {
      expect((await engine.info()).engineVersion).toBe('stand-in');
    } finally {
      engine.stop();
    }
  });

  it('tells the sample case’s story when there is one, and its own until then', async () => {
    const story = await expectedStory();
    const engine = await pickEngine('');
    try {
      const opened = await engine.openFilm('front', story.films.front.file, null);
      expect(opened.facts).toEqual(story.films.front.facts);
    } finally {
      engine.stop();
    }
  });
});
