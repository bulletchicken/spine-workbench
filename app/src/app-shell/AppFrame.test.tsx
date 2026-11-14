import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { act, useLayoutEffect, type ReactNode } from 'react';
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { saidNow } from '@/building-blocks/toast/say';
import { find, focusOn, pressKey, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { caseStore } from '@/case-state/case-store';
import { emptyCase } from '@/case-state/empty-case';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { filmRecordFrom } from '@/open-and-save/film-record';
import { startStandInEngine } from '@/engine-room/stand-in/stand-in-engine';
import { EngineProvider } from '@/engine-room/use-engine';
import { popoverOpen } from '@/on-this-device/popover-open';
import type { StepContract } from '@/steps/step-contract';
import { activeTool } from '@/view-state/active-tool';
import { caseSheetOpen } from '@/view-state/case-sheet-open';
import { currentStep } from '@/view-state/current-step';
import { dockNow, useShowInDock } from '@/view-state/dock-now';
import { preferences } from '@/view-state/preferences';
import { faultWords } from '@/words/faults';
import { PREVIEW_NOTICE } from '@/words/preview-notice';
import { TopBar } from './TopBar';
import { AppFrame } from './AppFrame';
import type { RoomParts } from './room-parts';

// The frame's tests stand alone while other room parts are being built.
vi.mock('./room-parts', () => ({ findRoomParts: () => ({ Bench: null, CaseSheet: null, OpeningCase: null }) }));

const NO_PARTS: RoomParts = { Bench: null, CaseSheet: null, OpeningCase: null };
const read = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8');
const nothing = () => undefined;
function fixtureStep(number: number, slug: string, title: string, more: Partial<StepContract> = {}): StepContract {
  function Screen() {
    useShowInDock({ scene: 'ready', sentence: `This is ${title}.`, primary: { label: 'Find the spine', press: nothing } });
    return null;
  }
  return { number, slug, title, Screen, isReady: () => true, canLeave: () => false, ...more };
}
const STEPS = [fixtureStep(1, 'films', 'Films'), fixtureStep(2, 'find', 'Find')];
let systemReduced = false;
const motionListeners = new Set<() => void>();
const engines: { stop(): void }[] = [];

function mount(steps: readonly StepContract[] = [], parts: RoomParts = NO_PARTS, given?: EngineClient) {
  const engine = given ?? startStandInEngine();
  if (given === undefined) engines.push(engine as EngineClient & { stop(): void });
  const page = putOnThePage(<EngineProvider engine={engine}><AppFrame steps={steps} parts={parts} /></EngineProvider>);
  return { page, engine };
}
async function ready(engine: EngineClient) {
  await act(async () => { await engine.info().catch(nothing); });
}
const button = (label: string) => {
  const found = [...document.querySelectorAll<HTMLButtonElement>('.steps button')].find((each) => each.textContent === label);
  if (found === undefined) throw new Error(`no step named ${label}`);
  return found;
};

beforeEach(() => {
  systemReduced = false;
  motionListeners.clear();
  vi.stubGlobal('matchMedia', vi.fn(() => ({
    matches: systemReduced,
    addEventListener: (_: string, listener: () => void) => { motionListeners.add(listener); },
    removeEventListener: (_: string, listener: () => void) => { motionListeners.delete(listener); },
  })));
  currentStep.getState().go('films');
  caseStore.getState().apply([{ kind: 'open-case', case: emptyCase() }]);
  caseSheetOpen.getState().setOpen(false);
  popoverOpen.getState().setOpen(false);
  activeTool.getState().pick(null);
  preferences.getState().setMotion('system', false);
  dockNow.getState().show(null);
});
afterEach(() => { for (const engine of engines.splice(0)) engine.stop(); });
afterAll(() => { vi.unstubAllGlobals(); });

describe('the one stage frame', () => {
  it('stands whole without steps or a bench: glow, frame, bar, stage, dock and notice', async () => {
    const { engine } = mount();
    await ready(engine);
    expect(find('.app-frame > .glow').getAttribute('aria-hidden')).toBe('true');
    const frame = find('.app-frame > .frame');
    expect(frame.querySelector(':scope > header.bar.glass')).not.toBeNull();
    expect(find('main.stage').getAttribute('aria-label')).toBe('The films');
    expect(find('main.stage').children).toHaveLength(0);
    expect(find('footer.dock.glass').getAttribute('aria-label')).toBe('What to do now');
    expect(document.querySelectorAll('.scene')).toHaveLength(13);
    expect(document.querySelector('.scene.on')).toBeNull();
    expect(frame.querySelector(':scope > p.notice')).not.toBeNull();
    expect(find('.app-frame').getAttribute('data-step')).toBe('');
    expect(document.querySelector('.guide-column, .status-line, .case-title, .top-bar-save')).toBeNull();
    expect(find('.brand-mark').getAttribute('viewBox')).toBe('0 0 20 20');
    expect(document.querySelectorAll('.brand-mark rect')).toHaveLength(3);
  });

  it('shows Save case before the trust badge only while a present film has found vertebrae', async () => {
    const { engine } = mount(STEPS);
    await ready(engine);
    const story = smallStory();
    expect(document.querySelector('.bar-save')).toBeNull();
    act(() => { caseStore.getState().apply([{ kind: 'set-film', film: filmRecordFrom(story.films.front.facts) }]); });
    expect(document.querySelector('.bar-save')).toBeNull();
    act(() => {
      const state = caseStore.getState();
      const marks = story.pass.marks.find((m) => m.film === 'front');
      if (marks === undefined) throw new Error('missing Front marks');
      state.apply([{ kind: 'open-case', case: { ...state.case, marks: [marks], nextVertebraId: 100 } }]);
    });
    expect(find('.bar-save').textContent).toBe('Save case');
    expect(find('.bar-save').nextElementSibling).toBe(find('.badge'));
    act(() => { caseStore.getState().apply([{ kind: 'remove-film', film: 'front' }]); });
    expect(document.querySelector('.bar-save')).toBeNull();
  });

  it('keeps Save focused while busy and accepts no second press', () => {
    const onSave = vi.fn();
    const bar = (saving: boolean) => <TopBar rail={[]} onGoToStep={nothing} requests={{ sent: 0, blocked: 0 }} promiseOpen={false} onPromiseOpenChange={nothing} onSave={onSave} saving={saving} />;
    const page = putOnThePage(bar(false));
    const save = find('.bar-save') as HTMLButtonElement;
    focusOn(save);
    page.show(bar(true));
    expect(document.activeElement).toBe(save);
    expect(save.disabled).toBe(false);
    expect(save.getAttribute('aria-disabled')).toBe('true');
    pressOn(save);
    expect(onSave).not.toHaveBeenCalled();
    page.show(bar(false));
    expect(document.activeElement).toBe(save);
    pressOn(save);
    expect(onSave).toHaveBeenCalledTimes(1);
  });

  it('keeps the approved notice word for word and the browser’s engine hooks', async () => {
    const { engine } = mount();
    const notice = find('p.notice');
    expect(notice.firstChild?.textContent).toBe(PREVIEW_NOTICE.short);
    expect(notice.getAttribute('data-testid')).toBe('engine-status');
    expect(notice.getAttribute('data-status')).toBe('starting');
    expect(document.querySelector('[data-testid="engine-version"]')).toBeNull();
    await ready(engine);
    expect(notice.getAttribute('data-status')).toBe('ready');
    expect(find('[data-testid="engine-version"]').closest('.spoken-only')).not.toBeNull();
    expect(find('[data-testid="engine-version"]').textContent).toBe((await engine.info()).engineVersion);
    expect(document.querySelectorAll('[data-testid="engine-status"]')).toHaveLength(1);
    expect(document.querySelectorAll('[data-testid="engine-version"]')).toHaveLength(1);
  });

  it('puts an engine failure in the dock, with no action or toast', async () => {
    const before = saidNow();
    const engine = { info: () => Promise.reject(new EngineFailure({ code: 'engine-load-failed', message: 'no engine', detail: null })) } as unknown as EngineClient;
    mount(STEPS, NO_PARTS, engine);
    await ready(engine);
    expect(find('p.notice').getAttribute('data-status')).toBe('failed');
    expect(find('.scene.on').getAttribute('data-scene')).toBe('engine');
    expect(find('.scene.on h2').textContent).toBe(`${faultWords('engine-load-failed').title}.`);
    expect(find('.scene.on p').textContent).toBe('Reload the page to try again.');
    expect(find('.scene.on').querySelector('button')).toBeNull();
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    expect(saidNow()).toBe(before);
  });

  it('keeps Place distinct from Films and returns to the 3D page state', async () => {
    const { engine } = mount([fixtureStep(3, 'three', '3D'), fixtureStep(4, 'place', 'Place')]);
    await ready(engine);
    act(() => { currentStep.getState().go('place'); });
    expect(document.documentElement.getAttribute('data-moment')).toBe('place');
    act(() => { currentStep.getState().go('three'); });
    expect(document.documentElement.getAttribute('data-moment')).toBe('three');
  });

  it('shows the first available step and hands its words to the single dock outside the screen', async () => {
    const steps = [fixtureStep(2, 'size-and-facing', 'Films'), fixtureStep(4, 'find', 'Find')];
    const { engine } = mount(steps);
    await ready(engine);
    expect(find('.app-frame').getAttribute('data-step')).toBe('facing');
    expect(document.documentElement.getAttribute('data-moment')).toBe('films');
    expect(button('Films').getAttribute('aria-current')).toBe('true');
    expect(find('.scene.on h2').textContent).toBe('This is Films.');
    act(() => { currentStep.getState().go('find'); });
    expect(find('.app-frame').getAttribute('data-step')).toBe('find');
    expect(document.documentElement.getAttribute('data-moment')).toBe('find');
    expect(find('.scene.on h2').textContent).toBe('This is Find.');
    expect(document.querySelectorAll('footer.dock')).toHaveLength(1);
  });

  it('lets a step behind be pressed, and keeps a step ahead disabled until its way opens', async () => {
    const steps = [fixtureStep(1, 'films', 'Films', { canLeave: (c) => c.nextVertebraId > 1 }), fixtureStep(2, 'find', 'Find', { canLeave: () => true })];
    const { engine } = mount(steps);
    await ready(engine);
    expect([...document.querySelectorAll('.steps button')].map((each) => each.textContent)).toEqual(['Films', 'Find', '3D']);
    expect(button('Find').disabled).toBe(true);
    pressOn(button('Find'));
    expect(currentStep.getState().slug).toBe('films');
    act(() => { caseStore.getState().apply([{ kind: 'open-case', case: { ...emptyCase(), nextVertebraId: 2 } }]); });
    expect(button('Find').disabled).toBe(false);
    pressOn(button('Find'));
    expect(button('Find').getAttribute('aria-current')).toBe('true');
    expect(find('.steps').style.getPropertyValue('--at')).toBe('1');
    expect(button('Films').disabled).toBe(false);
    pressOn(button('Films'));
    expect(currentStep.getState().slug).toBe('films');
    expect(button('3D').disabled).toBe(true);
    pressOn(button('3D'));
    expect(currentStep.getState().slug).toBe('films');
  });

  it('draws a registered step after Find once and follows its availability and destination', async () => {
    const steps = [
      fixtureStep(1, 'films', 'Films', { canLeave: () => true }),
      fixtureStep(2, 'find', 'Find', { canLeave: () => true }),
      fixtureStep(3, 'spine', '3D', { canLeave: () => true, isReady: (c) => c.nextVertebraId > 1 }),
    ];
    const { engine } = mount(steps);
    await ready(engine);
    expect([...document.querySelectorAll('.steps button')].map((each) => each.textContent)).toEqual(['Films', 'Find', '3D']);
    expect(button('3D').disabled).toBe(true);
    pressOn(button('3D'));
    expect(currentStep.getState().slug).toBe('films');
    act(() => { caseStore.getState().apply([{ kind: 'open-case', case: { ...emptyCase(), nextVertebraId: 2 } }]); });
    expect(button('3D').disabled).toBe(false);
    pressOn(button('3D'));
    expect(currentStep.getState().slug).toBe('spine');
    expect(button('3D').getAttribute('aria-current')).toBe('true');
    expect(find('.steps').style.getPropertyValue('--at')).toBe('2');
  });

  it('orders focusable controls as steps, badge, stage, dock', async () => {
    function Bench() { return <button type="button">Choose the Front film</button>; }
    function Screen() {
      useShowInDock({ scene: 'add', sentence: 'Add the four films.', helper: { kind: 'words', words: [{ link: 'choose them', press: nothing } satisfies { link: string; press: () => void }] }, quiet: { label: 'Use the sample case', press: nothing }, primary: { label: 'Find the spine', press: nothing } });
      return null;
    }
    const { engine } = mount([fixtureStep(1, 'films', 'Films', { Screen })], { ...NO_PARTS, Bench });
    await ready(engine);
    const order = [...document.querySelectorAll<HTMLButtonElement>('button')].filter((each) => !each.disabled && each.closest('[inert]') === null).map((each) => each.textContent);
    expect(order).toEqual(['Films', 'On this device only', 'Choose the Front film', 'choose them', 'Use the sample case', 'Find the spine']);
  });

  it('follows both motion preferences and keeps the tool attribute', async () => {
    const { engine } = mount();
    await ready(engine);
    expect(find('.app-frame').getAttribute('data-motion')).toBe('full');
    act(() => { preferences.getState().setMotion('reduced', false); });
    expect(find('.app-frame').getAttribute('data-motion')).toBe('reduced');
    act(() => { preferences.getState().setMotion('system', false); systemReduced = true; for (const listener of motionListeners) listener(); });
    expect(find('.app-frame').getAttribute('data-motion')).toBe('reduced');
    act(() => { systemReduced = false; for (const listener of motionListeners) listener(); activeTool.getState().pick('region-wand'); });
    expect(find('.app-frame').getAttribute('data-motion')).toBe('full');
    expect(find('.app-frame').getAttribute('data-tool')).toBe('region');
  });

  it('mounts the bench and sheet in the stage once, and wraps the keyed screen outside the footer', async () => {
    let mounts = 0;
    function Bench() { useLayoutEffect(() => { mounts += 1; }, []); return <div id="bench" />; }
    function CaseSheet() { return <div id="case-sheet" />; }
    function OpeningCase({ children }: { children: ReactNode }) { return <div id="opening-case">{children}</div>; }
    const { engine } = mount(STEPS, { Bench, CaseSheet, OpeningCase });
    await ready(engine);
    const bench = find('main.stage > #bench');
    expect(find('main.stage > #case-sheet')).not.toBeNull();
    expect(find('#opening-case').closest('footer')).toBeNull();
    expect(find('#opening-case').nextElementSibling).toBe(find('.dock'));
    act(() => { currentStep.getState().go('find'); });
    expect(find('main.stage > #bench')).toBe(bench);
    expect(mounts).toBe(1);
    expect(find('.scene.on h2').textContent).toBe('This is Find.');
  });

  it('stands the dock primary down while the case sheet is open', async () => {
    const { engine } = mount(STEPS);
    await ready(engine);
    const primary = find('.scene.on button');
    expect(primary.className).toBe('primary');
    act(() => { caseSheetOpen.getState().setOpen(true); });
    expect(primary.className).toBe('secondary');
    expect(pressKey('Enter').defaultPrevented).toBe(false);
  });

  it('opens the note from the keyboard, closes on Escape and returns focus to the badge', async () => {
    const { engine } = mount(STEPS);
    await ready(engine);
    const badge = find('.badge');
    focusOn(badge);
    expect(pressKey('Enter').defaultPrevented).toBe(false);
    pressOn(badge);
    const note = find('.frame > .note.glass');
    expect(note.classList.contains('open')).toBe(true);
    expect(note.closest('.bar')).toBeNull();
    expect(document.activeElement).toBe(note);
    expect(note.textContent).toContain('Your films never leave this computer.');
    pressKey('Escape');
    expect(note.classList.contains('open')).toBe(false);
    expect(note.hasAttribute('inert')).toBe(true);
    expect(document.activeElement).toBe(badge);
  });

  it('takes no undo, redo or Save keys', async () => {
    const { engine } = mount(STEPS);
    await ready(engine);
    for (const init of [{ metaKey: true }, { ctrlKey: true }]) {
      for (const key of ['z', 's']) expect(pressKey(key, init).defaultPrevented).toBe(false);
      expect(pressKey('Z', { ...init, shiftKey: true }).defaultPrevented).toBe(false);
    }
    expect(caseSheetOpen.getState().open).toBe(false);
  });

  it('adds no minimum width or replacement layout rules', () => {
    expect(read('./AppFrame.css').trim()).toBe('.app-frame { position: absolute; inset: 0; overflow: hidden; }');
  });
});
