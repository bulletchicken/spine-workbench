// Which engine the screens are handed.
//
// The production build always gets the real engine: the crew of workers
// running the compiled Rust. In development the stand-in answers instead,
// so screens can be built and looked at before the engine can do what
// they ask, unless the address asks for the real one with `?engine=real`.
// The stand-in tells the sample case's story (`sample-case/story.ts`,
// found by a glob so this file never waits for it) or, until that file
// exists, a small story of its own; `?slow` runs its automatic pass at
// the slow pace.
//
// Everything about the stand-in sits inside the development branch, so
// the production build leaves it out entirely: a browser test fails if
// any of its code reaches the bundle.

import type { StandInStory } from '@/sample-case/story-shape';
import { startRealEngine } from './crew/start-real-engine';
import type { EngineClient } from './engine-client';

/** An engine and the way to close its workers when the page is done with it. */
export type RunningEngine = EngineClient & { stop(): void };

/** The engine for this page; `address` is its query string. */
export async function pickEngine(address: string): Promise<RunningEngine> {
  if (import.meta.env.DEV) {
    const query = new URLSearchParams(address);
    if (query.get('engine') !== 'real') {
      const sampleStory = Object.values(import.meta.glob<StandInStory>('../sample-case/story.ts', { import: 'default' }))[0];
      const [{ startStandInEngine }, story] = await Promise.all([import('./stand-in/stand-in-engine'), sampleStory?.() ?? null]);
      return startStandInEngine({ story, scripts: { pace: query.has('slow') ? 'slow' : 'normal' } });
    }
  }
  return startRealEngine();
}
