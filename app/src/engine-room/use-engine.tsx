// How screens reach the engine: the app shell wraps everything in an
// EngineProvider holding the engine pick-engine.ts chose, and any part
// below asks for it with useEngine(). Screens never see which engine it
// is.

import { createContext, use, type ReactNode } from 'react';
import type { EngineClient } from './engine-client';

const EngineContext = createContext<EngineClient | null>(null);

export function EngineProvider({ engine, children }: { engine: EngineClient; children: ReactNode }) {
  return <EngineContext value={engine}>{children}</EngineContext>;
}

/** The engine the nearest EngineProvider holds. */
export function useEngine(): EngineClient {
  const engine = use(EngineContext);
  if (engine === null) throw new Error('useEngine is called outside an EngineProvider');
  return engine;
}
