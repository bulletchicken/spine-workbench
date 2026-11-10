// How an engine worker starts: it waits for the compiled engine, makes an
// instance with `initSync`, checks the engine was built from the same
// shapes as the app, and reports `ready` with the engine's info.
//
// This worker has no job handlers yet, so every `run` is answered
// `failed` with `engine-out-of-sync`, as the protocol says for a job the
// worker does not know.

import { SHAPES_FINGERPRINT, type EngineError, type EngineInfo } from '@/shapes';
import type { FromWorker, ToWorker, WorkerRole } from '../messages';
import { engine_info, initSync, last_panic } from '../wasm-build/wasm_bridge';

/** The parts of a dedicated worker's global scope that boot uses. */
export interface WorkerPort {
  postMessage(message: FromWorker, transfer?: Transferable[]): void;
  addEventListener(type: 'message', listener: (event: MessageEvent<ToWorker>) => void): void;
  close(): void;
}

/**
 * This worker's own global scope as a WorkerPort. Each part is reached by
 * name, since the network ban never lets the global object itself be
 * passed around.
 */
export function ownScope(): WorkerPort {
  return {
    postMessage: (message, transfer = []) => {
      self.postMessage(message, { transfer });
    },
    addEventListener: (type, listener) => {
      self.addEventListener(type, listener);
    },
    close: () => {
      self.close();
    },
  };
}

export function startWorker(port: WorkerPort, role: WorkerRole): void {
  let started = false;

  port.addEventListener('message', ({ data: message }) => {
    switch (message.kind) {
      case 'boot':
        if (started) return;
        started = true;
        boot(port, message.module);
        return;
      case 'run':
        port.postMessage({
          kind: 'failed',
          id: message.id,
          error: {
            code: 'engine-out-of-sync',
            message: `the ${role} worker has no handler for this job`,
            detail: message.job,
          },
        });
        return;
      case 'cancel':
        // Nothing here runs long enough to cancel.
        return;
    }
  });
}

function boot(port: WorkerPort, module: WebAssembly.Module): void {
  let info: EngineInfo;
  try {
    initSync({ module });
  } catch (cause) {
    crash(port, { code: 'engine-load-failed', message: 'the engine could not start', detail: String(cause) });
    return;
  }
  try {
    info = engine_info() as EngineInfo;
  } catch (cause) {
    crash(port, fromThrown(cause));
    return;
  }
  if (info.shapesFingerprint !== SHAPES_FINGERPRINT) {
    crash(port, {
      code: 'engine-out-of-sync',
      message: 'the engine was built from different shapes than the app',
      detail: `engine ${info.shapesFingerprint}, app ${SHAPES_FINGERPRINT}`,
    });
    return;
  }
  port.postMessage({ kind: 'ready', info });
}

/** Reports a failure the worker cannot go on from, and closes it. */
function crash(port: WorkerPort, error: EngineError): void {
  port.postMessage({ kind: 'crashed', error });
  port.close();
}

/**
 * What an engine call threw, as an EngineError. A panic reaches here as a
 * WebAssembly trap; the panic hook stored its message before the trap.
 */
function fromThrown(thrown: unknown): EngineError {
  if (thrown instanceof WebAssembly.RuntimeError) {
    return { code: 'engine-crashed', message: 'the engine stopped', detail: last_panic() ?? thrown.message };
  }
  if (isEngineError(thrown)) return thrown;
  return { code: 'engine-crashed', message: 'the engine threw something unexpected', detail: String(thrown) };
}

function isEngineError(value: unknown): value is EngineError {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as Partial<EngineError>).code === 'string' &&
    typeof (value as Partial<EngineError>).message === 'string'
  );
}
