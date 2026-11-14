import { act, StrictMode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AppFrame } from '@/app-shell/AppFrame';
import { takeIn } from '@/bench/film-intake';
import { findReplay } from '@/auto-pass/find-replay-clock';
import { caseStore } from '@/case-state/case-store';
import { emptyCase } from '@/case-state/empty-case';
import type { FrameStep } from '@/choreography/one-loop';
import { EngineFailure } from '@/engine-room/engine-client';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { DEFAULT_SCRIPTS, startStandInEngine } from '@/engine-room/stand-in/stand-in-engine';
import { EngineProvider } from '@/engine-room/use-engine';
import { filmRecordFrom } from '@/open-and-save/film-record';
import { currentStep } from '@/view-state/current-step';
import { dockNow } from '@/view-state/dock-now';
import { filmIntake } from '@/view-state/film-intake';
import { findState } from '@/view-state/find-state';
import { passRequest } from '@/view-state/pass-request';
import { findRun } from './find-run';
import { clearSpineBuild } from '@/bench/three/build-spine';
vi.mock('@/spine-3d/bones/load-bones', () => ({ loadBones: () => Promise.resolve([{ level: 'L3' }]) }));

vi.mock('@/sample-films/sample-films', () => ({ sampleFilmFiles: () => [], sampleFilmFilesFor: () => Promise.resolve([]) }));
vi.mock('@/choreography/timing', () => ({
  durationOnPage: (name: string) => ({ '--dur-scene': 640, '--stagger': 40, '--dur-fade': 160, '--dur-quick': 140, '--dur-light-lead': 700, '--dur-light': 4400, '--dur-hold': 560, '--hold-leave': 260 })[name],
  numberOnPage: (name: string) => ({ '--light-h': 200, '--light-edge': 60, '--film-h': 1300 })[name],
}));
const loop = vi.hoisted(() => ({ steps: new Set<FrameStep>() }));
vi.mock('@/choreography/one-loop', () => ({ joinLoop: (step: FrameStep) => { loop.steps.add(step); return () => { loop.steps.delete(step); }; } }));
globalThis.IS_REACT_ACT_ENVIRONMENT = true;
const story = smallStory();
let engine: ReturnType<typeof startStandInEngine>;
let host: HTMLDivElement;
let root: Root;
async function tick(ms: number) { await act(async () => { await vi.advanceTimersByTimeAsync(ms); }); }
function press(label: string) {
  const button = [...host.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent === label && !b.closest('[inert]'));
  if (button === undefined) throw new Error(`no ${label}`);
  act(() => { button.click(); });
}
function stepButton(label: string) {
  const button = [...host.querySelectorAll<HTMLButtonElement>('nav.steps button')].find((b) => b.textContent === label);
  if (button === undefined) throw new Error(`no step ${label}`);
  return button;
}
function escape() { act(() => { document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); }); }
function play() { act(() => { for (let frame = 0; frame < 60; frame += 1) for (const step of [...loop.steps]) step(100, frame * 100); }); }

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
  vi.stubGlobal('matchMedia', () => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined }));
  engine = startStandInEngine({ story, scripts: { films: { openMs: 0, displayPixelsMs: 0, makeUpMs: 0 }, find: { ...DEFAULT_SCRIPTS.find, normalMs: 40, placePointsMs: 0 } } });
  clearSpineBuild();
  findRun.stop(); loop.steps.clear(); passRequest.getState().take(); filmIntake.getState().clear();
  caseStore.getState().apply([{ kind: 'open-case', case: emptyCase() }]); currentStep.getState().go('films');
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
});
afterEach(() => { act(() => { root.unmount(); }); findRun.stop(); engine.stop(); host.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.useRealTimers(); });
async function mount() {
  await act(async () => { root.render(<StrictMode><EngineProvider engine={engine}><AppFrame parts={{ Bench: null, CaseSheet: null, OpeningCase: null }} /></EngineProvider></StrictMode>); await Promise.resolve(); });
}

describe('Find through the frame', () => {
  it('retries the same Front-only search after a multi-film failure, and goes back with the films', async () => {
    caseStore.getState().apply([{ kind: 'open-case', case: { ...emptyCase(), films: Object.values(story.films).map((film) => filmRecordFrom(film.facts)) } }]);
    filmIntake.getState().source('yours');
    const failure = (code: 'pass-no-spine-found' | 'engine-crashed') => new EngineFailure({ code, message: '', detail: null });
    const called = vi.spyOn(engine, 'runAutoPass').mockRejectedValueOnce(failure('pass-no-spine-found')).mockRejectedValue(failure('engine-crashed'));
    passRequest.getState().request(); currentStep.getState().go('find'); await mount(); await tick(0);
    expect(dockNow.getState().spec?.sentence).toBe('One film showed no spine.');
    press('Search Front alone'); await tick(0);
    expect(dockNow.getState().spec?.sentence).toBe('The search stopped before it finished.');
    press('Try again'); await tick(0);
    expect(called.mock.calls.map(([input]) => input.films.map((film) => film.kind))).toEqual([['front', 'side', 'bend-left', 'bend-right'], ['front'], ['front']]);
    press('Back to the films'); await tick(140);
    expect(caseStore.getState().case.films).toHaveLength(4);
    expect(findState.getState().phase).toBe('idle');
  });
  it('takes the made-up case from Films to found, then reviews each doubtful level', async () => {
    await mount(); press('Use the sample case'); await tick(1000);
    const steps = [...host.querySelectorAll<HTMLButtonElement>('nav.steps button')];
    expect(steps.map((button) => button.textContent)).toEqual(['Films', 'Find', '3D']);
    expect(steps.at(-1)?.disabled).toBe(true);
    expect(caseStore.getState().case.films).toHaveLength(4);
    const called = vi.spyOn(engine, 'runAutoPass'); press('Find the spine');
    expect(dockNow.getState().spec?.sentence).toBe('Finding the spine…');
    await tick(1000);
    expect(called).toHaveBeenCalledOnce();
    expect(called.mock.calls[0]?.[0].frontFinder).toBe('hand-built');
    expect(findState.getState().phase).toBe('replaying');
    expect(host.querySelector<HTMLButtonElement>('[data-scene="finding"].on .primary')?.disabled).toBe(true);
    play(); expect(findState.getState().phase).toBe('found');
    expect(loop.steps.size).toBe(0);
    const saved = JSON.stringify(caseStore.getState().case);
    const review = dockNow.getState().spec?.quiet?.label; if (review === undefined) throw new Error('no review'); press(review);
    const levels = [...findState.getState().reviewing];
    for (const level of levels) {
      expect(dockNow.getState().spec?.sentence).toBe(`${level} needs a look.`);
      press('Looks right'); press('Looks right');
      expect(findState.getState().looked.filter((looked) => looked === level)).toHaveLength(1);
      await tick(559); expect(findState.getState().holding).toBe(true);
      await tick(1);
    }
    expect(dockNow.getState().spec?.helper).toEqual({ kind: 'words', words: ['All of them look right.'] });
    expect(JSON.stringify(caseStore.getState().case)).toBe(saved);
    expect(findReplay.getState().allShown).toBe(true);
    const fitting = vi.spyOn(engine, 'fitSpine');
    press('Build the 3D spine');
    expect(dockNow.getState().spec?.sentence).toBe('Building the 3D spine…');
    await tick(1000);
    expect(currentStep.getState().slug).toBe('three');
    expect(dockNow.getState().spec?.sentence).toBe('The spine in 3D.');
    expect([...host.querySelectorAll('.scene.on .views button')].map((b) => b.textContent)).toEqual(['Front', 'Side', 'Top']);
    act(() => { stepButton('Find').click(); });
    expect(currentStep.getState().slug).toBe('find');
    expect(stepButton('3D').disabled).toBe(false);
    press('Build the 3D spine'); await tick(0);
    expect(currentStep.getState().slug).toBe('three');
    expect(fitting).toHaveBeenCalledTimes(1);
  });
  it('leaves a holding review cleanly on Escape or pressing Films, and cancels the next timer', async () => {
    const marks = structuredClone(story.pass.marks);
    const first = marks[0]?.vertebrae[0]; if (first === undefined) throw new Error('no outline'); first.confidence.band = 'low';
    caseStore.getState().apply([{ kind: 'open-case', case: { ...emptyCase(), films: Object.values(story.films).map((film) => filmRecordFrom(film.facts)), marks } }]);
    currentStep.getState().go('find'); await mount();
    const review = dockNow.getState().spec?.quiet?.label; if (review === undefined) throw new Error('no review');
    press(review);
    press('On this device only'); escape();
    expect(findState.getState().review).toBe(0);
    press('Looks right'); escape(); await tick(560);
    expect(findState.getState().review).toBeNull();
    expect(dockNow.getState().spec?.scene).toBe('found');
    const again = dockNow.getState().spec?.quiet?.label; if (again === undefined) throw new Error('no remaining review');
    press(again); press('Looks right'); press('Films'); await tick(560);
    expect(findState.getState().review).toBeNull();
    expect(currentStep.getState().slug).toBe('films');
    press('Start over'); await tick(260);
    expect(findState.getState().phase).toBe('idle');
    expect(findState.getState().looked).toEqual([]);
    expect(findReplay.getState().plan).toBeNull();
  });
  it('ends in found, not in a wait nothing will end, when Films and then Find are pressed during the replay', async () => {
    await mount(); press('Use the sample case'); await tick(1000);
    press('Find the spine'); await tick(1000);
    expect(findState.getState().phase).toBe('replaying');
    // Back in Find before the leaving fade (140 ms) has ended: the stopped replay is over.
    press('Films'); await tick(50); press('Find'); await tick(140);
    expect(currentStep.getState().slug).toBe('find');
    expect(findState.getState().phase).toBe('found');
    expect(dockNow.getState().spec?.scene).toBe('found');
    expect(findReplay.getState().plan).toBeNull();
    expect(loop.steps.size).toBe(0);
  });
  it('keeps Find shut while a new Front film is read, and never calls an unsearched film found', async () => {
    const front = story.films.front;
    await mount();
    const first = takeIn(engine, [new Blob([front.file.slice(0)])], null);
    await tick(10); await act(async () => { await first; });
    press('Find the spine'); await tick(1000); play();
    expect(findState.getState().phase).toBe('found');
    press('Films'); await tick(140);

    // A new film on the Front place, which the engine takes a while to read.
    let release: () => void = () => undefined;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const open = engine.openFilm.bind(engine);
    vi.spyOn(engine, 'openFilm').mockImplementation(async (...args) => { await gate; return open(...args); });
    const reading = takeIn(engine, [new Blob([front.file.slice(0)])], 'front');
    await tick(0);
    expect(dockNow.getState().spec?.sentence).toBe('Reading the film…');
    // The old outlines are still in the case, but the way forward is shut until the film has landed.
    expect(stepButton('Find').disabled).toBe(true);
    release(); await tick(10); await act(async () => { await reading; }); await tick(0);
    expect(caseStore.getState().case.marks.find((marks) => marks.film === 'front')).toBeUndefined();
    expect(stepButton('Find').disabled).toBe(true);
    expect(currentStep.getState().slug).toBe('films');
  });
  it('gives way to Films when the Front film is replaced under a finished search', async () => {
    const marks = structuredClone(story.pass.marks);
    const films = Object.values(story.films).map((film) => filmRecordFrom(film.facts));
    caseStore.getState().apply([{ kind: 'open-case', case: { ...emptyCase(), films, marks } }]);
    currentStep.getState().go('find'); await mount();
    expect(dockNow.getState().spec?.scene).toBe('found');
    const front = films.find((film) => film.kind === 'front'); if (front === undefined) throw new Error('no Front film');
    act(() => { caseStore.getState().apply([{ kind: 'set-film', film: front }]); });
    await tick(0);
    expect(caseStore.getState().case.marks.find((each) => each.film === 'front')).toBeUndefined();
    expect(currentStep.getState().slug).toBe('films');
    expect(findState.getState().phase).toBe('idle');
    expect(dockNow.getState().spec?.sentence).not.toMatch(/vertebrae found/);
  });
});
