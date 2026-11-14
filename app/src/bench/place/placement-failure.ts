import { EngineFailure } from '@/engine-room/engine-client';
import { PLACE_WORDS as W } from '@/words/place';
export interface PlacementFailure { kind: 'failed' | 'refused'; reason: string }
/** Only these worker faults can usefully be retried without changing the input. */
export function placementFailure(error: unknown): PlacementFailure | null {
  if (error instanceof EngineFailure) {
    if (error.error.code === 'pass-cancelled') return null;
    if (['engine-crashed', 'engine-out-of-memory', 'engine-load-failed'].includes(error.error.code)) return { kind: 'failed', reason: W.failed };
  }
  return { kind: 'refused', reason: W.refused };
}
