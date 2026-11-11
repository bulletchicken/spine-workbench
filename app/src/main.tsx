import { StrictMode, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { EngineFailure } from './engine-room/engine-client';
import { loadEngine } from './engine-room/load-engine';
import type { FromWorker, ToWorker } from './engine-room/messages';
import './look-and-feel/base.css';
import { faultWords } from './words/faults';
import { PREVIEW_NOTICE } from './words/preview-notice';
import type { EngineError, EngineInfo } from '@/shapes';

// The bare app: it starts one film worker and the spine worker, hands each
// the compiled engine, and shows the version they report.

type EngineState =
  | { status: 'starting' }
  | { status: 'ready'; info: EngineInfo }
  | { status: 'failed'; error: EngineError };

function bootWorker(worker: Worker, module: WebAssembly.Module): Promise<EngineInfo> {
  return new Promise((resolve, reject) => {
    worker.addEventListener('message', ({ data }: MessageEvent<FromWorker>) => {
      if (data.kind === 'ready') resolve(data.info);
      else if (data.kind === 'crashed') reject(new EngineFailure(data.error));
    });
    worker.addEventListener('error', (event) => {
      reject(new EngineFailure({ code: 'engine-load-failed', message: 'a worker did not start', detail: event.message }));
    });
    const boot: ToWorker = { kind: 'boot', module };
    worker.postMessage(boot);
  });
}

function startEngine(): { ready: Promise<EngineInfo>; stop(): void } {
  const workers = [
    new Worker(new URL('./engine-room/workers/film-worker/film.worker.ts', import.meta.url), { type: 'module' }),
    new Worker(new URL('./engine-room/workers/spine-worker/spine.worker.ts', import.meta.url), { type: 'module' }),
  ];
  const ready = loadEngine()
    .then((module) => Promise.all(workers.map((worker) => bootWorker(worker, module))))
    .then(([film]) => {
      if (film === undefined) throw new Error('no film worker');
      return film;
    });
  return {
    ready,
    stop: () => {
      for (const worker of workers) worker.terminate();
    },
  };
}

function App() {
  const [engine, setEngine] = useState<EngineState>({ status: 'starting' });

  useEffect(() => {
    const started = startEngine();
    let current = true;
    started.ready.then(
      (info) => {
        if (current) setEngine({ status: 'ready', info });
      },
      (cause: unknown) => {
        if (!current) return;
        const error: EngineError =
          cause instanceof EngineFailure
            ? cause.error
            : { code: 'engine-load-failed', message: 'the engine did not start', detail: String(cause) };
        setEngine({ status: 'failed', error });
      },
    );
    return () => {
      current = false;
      started.stop();
    };
  }, []);

  return (
    <main>
      <h1>Spine Workbench</h1>
      <p data-testid="engine-status" data-status={engine.status}>
        {engine.status === 'starting' && 'Starting the engine…'}
        {engine.status === 'ready' && (
          <>
            Engine <span data-testid="engine-version">{engine.info.engineVersion}</span> ready
          </>
        )}
        {engine.status === 'failed' && (
          <>
            {faultWords(engine.error.code).title}. {faultWords(engine.error.code).whatToDo}
          </>
        )}
      </p>
      <p>{PREVIEW_NOTICE.short}</p>
    </main>
  );
}

const root = document.getElementById('root');
if (root === null) throw new Error('index.html has no #root element');
const page = createRoot(root);

// In development, ?gallery shows every part in every state in place of the
// app. A production build has no gallery in it: this branch, and with it
// the only import of gallery/, is dropped when the build is made.
if (import.meta.env.DEV && new URLSearchParams(window.location.search).has('gallery')) {
  void import('./gallery/Gallery').then(({ Gallery }) => {
    page.render(
      <StrictMode>
        <Gallery />
      </StrictMode>,
    );
  });
} else {
  page.render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
