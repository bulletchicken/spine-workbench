import { Blob as FileBlob } from 'node:buffer';
import { StrictMode, act } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { find, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { caseStore } from '@/case-state/case-store';
import { startOver, clearFilms } from '@/bench/start-over';
import { takeInNextSample, takeIn } from '@/bench/film-intake';
import { EngineProvider } from '@/engine-room/use-engine';
import { startStandInEngine } from '@/engine-room/stand-in/stand-in-engine';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { Dock } from '@/guide/Dock';
import { currentStep } from '@/view-state/current-step';
import { useDockNow } from '@/view-state/dock-now';
import { filmIntake } from '@/view-state/film-intake';
import { filmOrigins } from '@/view-state/film-origins';
import { filmPixels } from '@/case-state/film-pixels';
import type { SampleCase } from '@/sample-films/sample-films';
import { SideFacing } from '@/bench/SideFacing';
import { FilmCredit } from '@/bench/FilmCredit';
import { passRequest } from '@/view-state/pass-request';
import step from './step';
import { FilmsScreen } from './FilmsScreen';

vi.mock('@/choreography/timing', () => ({ durationOnPage: () => 0 }));
const samples = vi.hoisted(() => ({ files: [] as { bytes(): ArrayBuffer }[], local: null as SampleCase | null }));
vi.mock('@/sample-films/sample-films', () => ({ sampleCaseFor: async (engine: { info(): Promise<{ engineVersion: string }> }) => (await engine.info()).engineVersion === 'stand-in' ? null : samples.local,
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
  samples.local = null;
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
    expect(find('.scene.on .quiet').textContent).toBe('Start over');
  });
  it('keeps the step contract open and allows leaving only with Front', () => {
    expect(step.number).toBe(1);
    expect(step.slug).toBe('films');
    expect(step.isReady()).toBe(true);
    expect(step.canLeave(caseStore.getState().case)).toBe(false);
  });
});

describe('the real sample case control, using stand-in films', () => {
  async function offer(local: SampleCase) {
    samples.local = local;
    vi.spyOn(engine, 'info').mockResolvedValue({ ...await engine.info(), engineVersion: '0.1.0' });
    mount();
    await until(() => { expect(find('.scene.on .quiet').textContent).toBe('Use the real sample case'); });
  }
  const file = (kind: keyof typeof story.films) => ({ bytes: () => story.films[kind].file.slice(0) });
  it('offers the case first, assigns all four slots, answers facing and goes round the real films only', async () => {
    samples.files = [file('front')];
    await offer({ films: { front: file('front'), side: file('side'), 'bend-left': file('bend-left'), 'bend-right': file('bend-right') }, sideFaces: 'right', credit: 'Made-up test films, CC0.' });
    const opened = vi.spyOn(engine, 'openFilm');
    pressOn(find('.scene.on .quiet'));
    await until(() => { expect(sentence()).toBe('All four films are in.'); });
    expect(opened.mock.calls.map(([kind]) => kind)).toEqual(['front', 'side', 'bend-left', 'bend-right']);
    expect(find('.scene.on .say p').textContent).toBe('Real films of one patient, kept on this machine.');
    expect(filmOrigins.getState().films.front?.from).toBe('real-case');
    expect(caseStore.getState().case.films.find((f) => f.kind === 'side')?.geometry).toMatchObject({ imageRight: 'anterior', confirmed: { facing: true } });
    const controls = putOnThePage(<SideFacing opening={filmPixels.getState().openings.side} />);
    const inputs = controls.host.querySelectorAll('input');
    expect((inputs[1] as HTMLInputElement).checked).toBe(true);
    pressOn(inputs[0] as HTMLInputElement);
    expect((inputs[0] as HTMLInputElement).checked).toBe(true);
    expect(filmOrigins.getState().credit).toBe('Made-up test films, CC0.');
    controls.takeDown();
    pressOn(find('.scene.on .quiet'));
    await until(() => { expect(sentence()).toBe('The sample film is in.'); });
    expect(filmOrigins.getState().credit).toBeNull();
    expect(find('.scene.on .quiet').textContent).toBe('Use the real sample case');
    pressOn(find('.scene.on .quiet'));
    await until(() => { expect(filmIntake.getState().from).toBe('real-case'); expect(sentence()).toBe('All four films are in.'); });
  });
  it('loads front alone without metadata, and preserves a missing side slot with bending films', async () => {
    await offer({ films: { front: file('front') } });
    pressOn(find('.scene.on .quiet'));
    await until(() => { expect(sentence()).toBe('The Front film is in.'); });
    expect(caseStore.getState().case.films.map((f) => f.kind)).toEqual(['front']);
    expect(filmOrigins.getState().credit).toBeNull();
    samples.local = { films: { front: file('front'), 'bend-right': file('bend-right') } };
    await act(async () => { filmIntake.getState().next(0); await takeInNextSample(engine); });
    expect(caseStore.getState().case.films.map((f) => f.kind)).toEqual(['front', 'bend-right']);
  });
  it('carries the unchanged credit through Films, Find and 3D, and clears it on Start over or replacement', async () => {
    await offer({ films: { front: file('front') }, credit: 'Drawn here — CC0.' });
    const creditPage = putOnThePage(<FilmCredit />);
    pressOn(find('.scene.on .quiet'));
    await until(() => { expect(creditPage.host.textContent).toBe('Films: Drawn here — CC0.'); });
    for (const step of ['find', 'three'] as const) {
      act(() => { currentStep.getState().go(step); });
      expect(creditPage.host.textContent).toBe('Films: Drawn here — CC0.');
    }
    act(() => { currentStep.getState().go('place'); });
    expect(creditPage.host.textContent).toBe('');
    await act(async () => { await startOver(engine); });
    expect(creditPage.host.textContent).toBe(''); expect(filmOrigins.getState().credit).toBeNull();
    await act(async () => { filmIntake.getState().next(0); await takeInNextSample(engine); });
    expect(creditPage.host.textContent).toBe('Films: Drawn here — CC0.');
    await act(async () => { await takeIn(engine, [new Blob([story.films.front.file])], 'front'); });
    expect(creditPage.host.textContent).toBe(''); expect(filmOrigins.getState().credit).toBeNull();
  });
});
