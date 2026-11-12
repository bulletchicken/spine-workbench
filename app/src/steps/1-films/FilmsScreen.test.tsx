import { Blob as FileBlob } from 'node:buffer';
import { StrictMode, act } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { find, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { caseStore } from '@/case-state/case-store';
import { clearFilms } from '@/bench/start-over';
import { takeIn } from '@/bench/film-intake';
import { EngineProvider } from '@/engine-room/use-engine';
import { startStandInEngine } from '@/engine-room/stand-in/stand-in-engine';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { Dock } from '@/guide/Dock';
import { currentStep } from '@/view-state/current-step';
import { useDockNow } from '@/view-state/dock-now';
import { filmIntake } from '@/view-state/film-intake';
import { passRequest } from '@/view-state/pass-request';
import step from './step';
import { FilmsScreen } from './FilmsScreen';

vi.mock('@/choreography/timing', () => ({ durationOnPage: () => 0 }));
const samples = vi.hoisted(() => ({ files: [] as { bytes(): ArrayBuffer }[] }));
vi.mock('@/sample-films/sample-films', () => ({
  sampleFilmFiles: () => samples.files,
  sampleFilmFilesFor: async (engine: { info(): Promise<{ engineVersion: string }> }) => (await engine.info()).engineVersion === 'stand-in' ? [] : samples.files,
}));
const story = smallStory();
let engine: ReturnType<typeof startStandInEngine>;
function LiveDock() { return <Dock spec={useDockNow()} />; }
function mount() {
  return putOnThePage(<StrictMode><EngineProvider engine={engine}><FilmsScreen /><LiveDock /></EngineProvider></StrictMode>);
}
function sentence(): string { return find('.scene.on h2').textContent; }
async function until(check: () => void) {
  await vi.waitFor(async () => {
    await act(async () => { await new Promise<void>((resolve) => { setTimeout(resolve, 10); }); });
    check();
  });
}
beforeEach(() => {
  vi.stubGlobal('Blob', FileBlob);
  engine = startStandInEngine({ story, scripts: { films: { openMs: 0, displayPixelsMs: 0, makeUpMs: 0 } } });
  clearFilms(engine);
  filmIntake.getState().next(0);
  currentStep.getState().go('films');
  samples.files = [];
});
afterEach(() => { engine.stop(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });

describe('the Films screen', () => {
  it('hands the frame the system parts and brings the made-up case when no sample films exist', async () => {
    mount();
    await until(() => { expect(sentence()).toBe('Add the four films.'); });
    expect(find('.scene.on .primary').hasAttribute('disabled')).toBe(true);
    expect(find('.scene.on .quiet').textContent).toBe('Use the sample case');
    expect(find('.scene.on .link').textContent).toBe('choose them');
    pressOn(find('.scene.on .quiet'));
    await until(() => { expect(sentence()).toBe('All four films are in.'); });
    expect(caseStore.getState().case.films).toHaveLength(4);
    expect(filmIntake.getState().from).toBe('made-up-case');
    expect(find('.scene.on .primary').hasAttribute('disabled')).toBe(false);
    expect(find('.scene.on .quiet').textContent).toBe('Start over');
  });
  it('keeps Find disabled without Front, then leaves one request and clears the notice on Find', async () => {
    mount();
    await act(async () => { await takeIn(engine, [new Blob([story.films.side.file])], 'side'); });
    expect(find('.scene.on .primary').hasAttribute('disabled')).toBe(true);
    await act(async () => { await takeIn(engine, [new Blob([story.films.front.file])], 'front'); });
    act(() => { filmIntake.getState().tell({ kind: 'search-stopped' }); });
    pressOn(find('.scene.on .primary'));
    expect(currentStep.getState().slug).toBe('find');
    expect(passRequest.getState().take()).toBe(true);
    expect(passRequest.getState().take()).toBe(false);
    expect(filmIntake.getState().notice).toBeNull();
  });
  it('empties the stage on Start over and preserves the sample round', async () => {
    mount();
    await act(async () => { await takeIn(engine, [new Blob([story.films.front.file])], 'front'); filmIntake.getState().next(2); });
    pressOn(find('.scene.on .quiet'));
    await until(() => { expect(sentence()).toBe('Add the four films.'); });
    expect(caseStore.getState().case.films).toEqual([]);
    expect(filmIntake.getState().nextSample).toBe(2);
  });
  it('offers local samples only when the engine says it can read them', async () => {
    samples.files = [{ bytes: () => story.films.front.file.slice(0) }];
    const page = mount();
    await until(() => { expect(find('.scene.on .quiet').textContent).toBe('Use the sample case'); });
    page.takeDown();
    const info = await engine.info();
    vi.spyOn(engine, 'info').mockResolvedValue({ ...info, engineVersion: '0.1.0' });
    mount();
    await until(() => { expect(find('.scene.on .quiet').textContent).toBe('Use a sample film'); });
    pressOn(find('.scene.on .quiet'));
    await until(() => { expect(sentence()).toBe('The sample film is in.'); });
    expect(find('.scene.on .quiet').textContent).toBe('Use the sample case');
  });
  it('keeps the step contract open and allows leaving only with Front', () => {
    expect(step.number).toBe(1);
    expect(step.slug).toBe('films');
    expect(step.isReady()).toBe(true);
    expect(step.canLeave(caseStore.getState().case)).toBe(false);
  });
});
