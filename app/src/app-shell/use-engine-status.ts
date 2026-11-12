// Whether the engine has started. The frame asks it for its info once;
// the answer, or the failure, is what the frame carries for the
// browser's guards and for the person.

import { useEffect, useState } from 'react';
import { EngineFailure } from '@/engine-room/engine-client';
import { useEngine } from '@/engine-room/use-engine';
import type { EngineError, EngineInfo } from '@/shapes';

export type EngineStatus = { status: 'starting' } | { status: 'ready'; info: EngineInfo } | { status: 'failed'; error: EngineError };

/** The engine's state, from `starting` to `ready` or `failed`. */
export function useEngineStatus(): EngineStatus {
  const engine = useEngine();
  const [status, setStatus] = useState<EngineStatus>({ status: 'starting' });

  useEffect(() => {
    let asking = true;
    engine.info().then(
      (info) => {
        if (asking) setStatus({ status: 'ready', info });
      },
      (cause: unknown) => {
        if (!asking) return;
        const error: EngineError =
          cause instanceof EngineFailure ? cause.error : { code: 'engine-load-failed', message: 'the engine did not start', detail: String(cause) };
        setStatus({ status: 'failed', error });
      },
    );
    return () => {
      asking = false;
    };
  }, [engine]);

  return status;
}
