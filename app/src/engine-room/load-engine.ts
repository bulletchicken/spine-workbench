// The one sanctioned fetch in the app: our own engine, from our own
// origin, compiled once.
//
// Vite emits the .wasm as a hashed file beside the page. This module
// fetches it, compiles it on the main thread and hands the compiled
// module to every worker at boot, where wasm-bindgen's `initSync` turns it
// into an instance. Workers never fetch, and wasm-bindgen's own loader,
// which would fetch, never runs.
//
// It is the only file the lint rule against network calls exempts, and a
// test fails if a second file is ever exempted. Compiling needs
// 'wasm-unsafe-eval' in the content policy, which allows WebAssembly but
// not JavaScript eval.

import { EngineFailure } from './engine-client';
import wasmUrl from './wasm-build/wasm_bridge_bg.wasm?url';

let compiled: Promise<WebAssembly.Module> | null = null;

/** The engine, compiled. Every call after the first gets the same module. */
export function loadEngine(): Promise<WebAssembly.Module> {
  compiled ??= compile();
  return compiled;
}

async function compile(): Promise<WebAssembly.Module> {
  const url = new URL(wasmUrl, location.href);
  if (url.origin !== location.origin) {
    throw loadFailed('the engine must come from the page’s own origin', url.origin);
  }

  let response: Response;
  try {
    response = await fetch(url, { credentials: 'same-origin' });
  } catch (cause) {
    throw loadFailed('the engine file could not be read', String(cause));
  }
  if (!response.ok) {
    throw loadFailed('the engine file could not be read', `HTTP ${String(response.status)}`);
  }

  try {
    // Streaming compiles while the file arrives, but browsers allow it only
    // when the host labels the file application/wasm.
    if (response.headers.get('content-type')?.split(';')[0]?.trim() === 'application/wasm') {
      return await WebAssembly.compileStreaming(response);
    }
    return await WebAssembly.compile(await response.arrayBuffer());
  } catch (cause) {
    throw loadFailed('the engine did not compile', String(cause));
  }
}

function loadFailed(message: string, detail: string): EngineFailure {
  return new EngineFailure({ code: 'engine-load-failed', message, detail });
}
