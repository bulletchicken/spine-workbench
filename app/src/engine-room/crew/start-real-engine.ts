// The real engine, started: the compiled engine from the one sanctioned
// loader, the spine worker and one spare film worker booting at once.

import { loadEngine } from '../load-engine';
import { Crew } from './crew';
import { RealEngine } from './real-engine';
import { spawnWorker } from './spawn-worker';

export function startRealEngine(): RealEngine {
  return new RealEngine(new Crew({ spawn: spawnWorker, engine: loadEngine }));
}
