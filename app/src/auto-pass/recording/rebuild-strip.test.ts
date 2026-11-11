import { EngineFailure, type EngineClient, type RunOptions } from '@/engine-room/engine-client';
import type { AutoPassInput, ErrorCode, WatchFrame } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { filmRecording, filmStrip, recordFrames } from './record-frames';
import { rebuildStrip } from './rebuild-strip';
import { testFilm, testFrames } from './test-frames';

/** An engine that sends the given frames one at a time and stops between them when asked to. */
function fakeEngine(frames: WatchFrame[], fails: ErrorCode | null = null) {
  const seen = { inputs: [] as AutoPassInput[], sent: [] as WatchFrame['kind'][], stopped: false };
  const engine: Pick<EngineClient, 'runAutoPass'> = {
    async runAutoPass(input: AutoPassInput, run?: RunOptions) {
      seen.inputs.push(input);
      for (const frame of frames) {
        await Promise.resolve();
        if (run?.signal?.aborted === true) {
          seen.stopped = true;
          throw new EngineFailure({ code: 'pass-cancelled', message: 'the pass was stopped', detail: null });
        }
        seen.sent.push(frame.kind);
        run?.onFrame?.(frame);
      }
      if (fails !== null) throw new EngineFailure({ code: fails, message: 'it went wrong', detail: null });
      return { caseVersion: input.caseVersion, marks: [], twoFilms: null, warnings: [] };
    },
  };
  return { engine, seen };
}

function wholeStrip() {
  const part = filmRecording(recordFrames(['front'], testFrames('front')), 'front');
  return part === null ? null : filmStrip(part);
}

describe('rebuilding a film’s strip', () => {
  it('gives the strip and stops the pass as soon as the barcode arrives', async () => {
    const { engine, seen } = fakeEngine(testFrames('front'));
    const strip = await rebuildStrip(engine, testFilm('front'), new AbortController().signal);
    expect(strip).toEqual(wholeStrip());
    expect(seen.sent.at(-1)).toBe('barcode');
    expect(seen.sent).not.toContain('sweep');
    expect(seen.stopped).toBe(true);
  });

  it('runs the pass for that one film alone', async () => {
    const { engine, seen } = fakeEngine(testFrames('front'));
    await rebuildStrip(engine, testFilm('front'), new AbortController().signal);
    expect(seen.inputs.map((input) => input.films.map((film) => film.kind))).toEqual([['front']]);
  });

  it('rejects with pass-cancelled when the caller stops it', async () => {
    const frames = testFrames('front');
    const caller = new AbortController();
    const { engine } = fakeEngine(frames);
    const watching: Pick<EngineClient, 'runAutoPass'> = {
      runAutoPass: (input, run) =>
        engine.runAutoPass(input, {
          ...run,
          onFrame(frame) {
            if (frame.kind === 'strip-rows') caller.abort();
            run?.onFrame?.(frame);
          },
        }),
    };
    const rebuilt = rebuildStrip(watching, testFilm('front'), caller.signal);
    await expect(rebuilt).rejects.toBeInstanceOf(EngineFailure);
    await expect(rebuilt).rejects.toMatchObject({ error: { code: 'pass-cancelled' } });
  });

  it('rejects at once for a caller that has already stopped', async () => {
    const caller = new AbortController();
    caller.abort();
    const { engine, seen } = fakeEngine(testFrames('front'));
    await expect(rebuildStrip(engine, testFilm('front'), caller.signal)).rejects.toMatchObject({ error: { code: 'pass-cancelled' } });
    expect(seen.sent).toEqual([]);
  });

  it('gives none when the pass ends without a barcode', async () => {
    const { engine } = fakeEngine(testFrames('front').filter((frame) => frame.kind !== 'barcode'));
    expect(await rebuildStrip(engine, testFilm('front'), new AbortController().signal)).toBeNull();
  });

  it('rejects with pass-cancelled when the engine cancels the pass itself', async () => {
    // Replacing or retiring the film cancels its pass before the barcode.
    const { engine } = fakeEngine(testFrames('front').filter((frame) => frame.kind !== 'barcode'), 'pass-cancelled');
    await expect(rebuildStrip(engine, testFilm('front'), new AbortController().signal)).rejects.toMatchObject({ error: { code: 'pass-cancelled' } });
  });

  it('passes on any other failure of the engine', async () => {
    const { engine } = fakeEngine(testFrames('front').filter((frame) => frame.kind !== 'barcode'), 'engine-crashed');
    await expect(rebuildStrip(engine, testFilm('front'), new AbortController().signal)).rejects.toMatchObject({ error: { code: 'engine-crashed' } });
  });
});
