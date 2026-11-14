import { act, StrictMode } from 'react';
import { afterEach, expect, it, vi } from 'vitest';
import { putOnThePage, pressOn, find } from '@/building-blocks/test-rig/on-the-page';
import { EngineProvider } from '@/engine-room/use-engine';
import type { EngineClient } from '@/engine-room/engine-client';
import { sampleReadings } from '@/engine-room/stand-in/spine-readings';
import { MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import { spineBuild, type AcceptedSpine } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import type { ReadingsInput } from '@/shapes';
import { Measurements } from './Measurements';
function source(): AcceptedSpine {
  return { key: 'readings-test', bones: [], fit: { ...MADE_UP_CASE.fit, frontFilm: { ...MADE_UP_CASE.fit.frontFilm, facingAssumed: true } }, input: { front: {
    geometry: MADE_UP_CASE.front.geometry, size: MADE_UP_CASE.front.size, markSpread: MADE_UP_CASE.front.markSpread, facingAssumed: true,
    marks: { film: 'front', vertebrae: [], naming: { flags: ['estimated'] } } as never,
  }, side: null, typical: 'built-in', twistReadings: [] } };
}
afterEach(() => { spineBuild.getState().clear(); threeState.getState().clear(); });
it('requests once across strict mounting and remounts, and renders signed rows, omissions and notes', async () => {
  const accepted = source(); spineBuild.getState().accept(accepted);
  const measureSpine = vi.fn((input: ReadingsInput) => Promise.resolve(sampleReadings(input)));
  const engine = { measureSpine } as unknown as EngineClient;
  const node = <StrictMode><EngineProvider engine={engine}><Measurements accepted={accepted} /></EngineProvider></StrictMode>;
  const page = putOnThePage(node);
  await act(async () => { await Promise.resolve(); });
  expect(measureSpine).toHaveBeenCalledTimes(1);
  expect(measureSpine).toHaveBeenCalledWith(expect.objectContaining({ fit: accepted.fit, front: accepted.input.front, versions: 32, seed: 7 }));
  expect(find('.panel').textContent).toContain('No side-to-side curve was found.');
  expect(find('.panel').textContent).toContain('Each band covers eight in ten of the spines that fit these marks.');
  expect(find('.panel').textContent).toContain('Names are estimates. Check where T1 is.');
  expect(find('.panel').textContent).toContain('Left and right assume the film is shown the usual way round.');
  expect(find('.measures li .m-band').textContent).toBe('± 0°');
  page.show(null); page.show(node);
  expect(measureSpine).toHaveBeenCalledTimes(1);
  page.takeDown();
});
it('retries the reading without requesting another fit', async () => {
  const accepted = source(); spineBuild.getState().accept(accepted);
  const measureSpine = vi.fn<(input: ReadingsInput) => ReturnType<EngineClient['measureSpine']>>()
    .mockRejectedValueOnce(new Error('unavailable')).mockImplementation((input) => Promise.resolve(sampleReadings(input)));
  const engine = { measureSpine } as unknown as EngineClient;
  const page = putOnThePage(<EngineProvider engine={engine}><Measurements accepted={accepted} /></EngineProvider>);
  await act(async () => { await Promise.resolve(); });
  expect(find('.panel').textContent).toContain('The measurements could not be read.');
  pressOn(find('.panel .m-about button'));
  await act(async () => { await Promise.resolve(); });
  expect(measureSpine).toHaveBeenCalledTimes(2);
  expect(find('.panel .measures')).not.toBeNull();
  expect(spineBuild.getState().accepted).toBe(accepted);
  page.takeDown();
});

it('shows the same-spine assumption in the panel and narrow sheet only when Side was used', async () => {
  const note = 'The front and side films are taken to show the same spine. Nothing here can check that.';
  try {
    for (const narrow of [false, true]) {
      vi.stubGlobal('innerWidth', narrow ? 700 : 1200);
      for (const sideUsed of [false, true]) {
        const accepted = source();
        accepted.key = `same-spine-${String(narrow)}-${String(sideUsed)}`;
        accepted.fit = { ...accepted.fit, sideFilm: sideUsed ? { ...accepted.fit.frontFilm } : null };
        spineBuild.getState().accept(accepted);
        threeState.getState().clear();
        threeState.getState().showSheet(narrow);
        const engine = { measureSpine: (input: ReadingsInput) => Promise.resolve(sampleReadings(input)) } as unknown as EngineClient;
        const page = putOnThePage(<EngineProvider engine={engine}><Measurements accepted={accepted} /></EngineProvider>);
        await act(async () => { await Promise.resolve(); });
        const panel = find('.measurements');
        expect(panel.classList.contains('sheet')).toBe(narrow);
        expect(panel.getAttribute('aria-hidden')).toBeNull();
        expect(find('.panel-note', panel).textContent.includes(note)).toBe(sideUsed);
        page.takeDown();
      }
    }
  } finally {
    vi.unstubAllGlobals();
  }
});
