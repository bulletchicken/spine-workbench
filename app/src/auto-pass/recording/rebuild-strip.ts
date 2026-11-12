// Runs the film stages for one film alone and stops, to give the ladder
// its strip back after a case is reopened (the case file holds no strip).
//
// It starts the automatic pass for that one film, keeps the strip's
// frames, and stops the pass the moment the film's `barcode` arrives, the
// last frame of the strip stage, so the chain search never runs. The
// pass's answer is never used and nothing touches the case. The caller
// keeps the strip with the film's stamp.

import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import type { AutoPassInput, Film, WatchFrame } from '@/shapes';
import type { FilmStrip } from './pass-recording';
import { filmRecording, filmStrip, recordFrame, startRecording } from './record-frames';

const STRIP_FRAMES: readonly WatchFrame['kind'][] = ['strip-shape', 'strip-rows', 'barcode'];

/**
 * The film's strip, or null if the pass ended without a whole one. An
 * abort of `signal` rejects with `pass-cancelled`, as the pass itself does,
 * and so does the engine cancelling the pass (the film replaced or
 * retired): only the stop at the barcode is swallowed.
 */
export async function rebuildStrip(engine: Pick<EngineClient, 'runAutoPass'>, film: Film, signal: AbortSignal): Promise<FilmStrip | null> {
  const stop = new AbortController();
  const stopWithCaller = () => {
    stop.abort();
  };
  if (signal.aborted) stop.abort();
  signal.addEventListener('abort', stopWithCaller, { once: true });

  // The chain options and ids reach only the chain search, which never
  // runs; the answer is dropped, so they are the defaults.
  const input: AutoPassInput = { caseVersion: 0, films: [film], options: { acquisition: 'unknown', population: 'adolescent', template: null }, firstVertebraId: 0, frontFinder: 'hand-built' };
  let recording = startRecording([film.kind]);
  const found: { strip: FilmStrip | null } = { strip: null };
  const onFrame = (frame: WatchFrame) => {
    if (found.strip !== null || frame.film !== film.kind || !STRIP_FRAMES.includes(frame.kind)) return;
    recording = recordFrame(recording, frame);
    if (frame.kind !== 'barcode') return;
    const part = filmRecording(recording, film.kind);
    found.strip = part === null ? null : filmStrip(part);
    stop.abort();
  };

  try {
    await engine.runAutoPass(input, { onFrame, signal: stop.signal });
  } catch (failure) {
    const ourStop = failure instanceof EngineFailure && failure.error.code === 'pass-cancelled' && stop.signal.aborted && !signal.aborted;
    if (!ourStop) throw failure;
  } finally {
    signal.removeEventListener('abort', stopWithCaller);
  }
  return found.strip;
}
