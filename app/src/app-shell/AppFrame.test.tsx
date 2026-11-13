// The frame, mounted on the stand-in engine: with no steps at all, then
// on a two-step fixture numbered 2 and 4, as the steps may land in any
// order. The fixture steps are plain components; the bench, the sheet and
// the opening screen are stand-ins handed in where the app would find
// its own.

import { act, useLayoutEffect, useRef, type ReactNode } from 'react';
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { saidNow } from '@/building-blocks/toast/say';
import { find, focusOn, pressKey, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { caseStore } from '@/case-state/case-store';
import { emptyCase } from '@/case-state/empty-case';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { startStandInEngine } from '@/engine-room/stand-in/stand-in-engine';
import { EngineProvider } from '@/engine-room/use-engine';
import { Guide } from '@/guide/Guide';
import { popoverOpen } from '@/on-this-device/popover-open';
import { caseAt } from '@/sample-case/stages';
import type { StepContract } from '@/steps/step-contract';
import { activeTool } from '@/view-state/active-tool';
import { caseSheetOpen } from '@/view-state/case-sheet-open';
import { currentStep } from '@/view-state/current-step';
import { pointer } from '@/view-state/pointer';
import { preferences } from '@/view-state/preferences';
import { savedState } from '@/view-state/saved-state';
import { faultWords } from '@/words/faults';
import { PREVIEW_NOTICE } from '@/words/preview-notice';
import { AppFrame } from './AppFrame';
import type { RoomParts } from './room-parts';

const NO_PARTS: RoomParts = { Bench: null, CaseSheet: null, OpeningCase: null };

function fixtureStep(number: number, slug: string, title: string, more: Partial<StepContract> = {}): StepContract {
  const Screen = () => <Guide spec={{ eyebrow: [title], sentence: `This is ${title}.`, lit: { label: `Go on from ${title}`, press: () => undefined } }} />;
  return { number, slug, title, Screen, isReady: () => true, canLeave: () => false, ...more };
}

/** The two steps a half-landed app might have: numbers 2 and 4, and no step 1. Adjust opens once a film is in the case. */
const TWO_STEPS = [fixtureStep(2, 'size-and-facing', 'Size & facing'), fixtureStep(4, 'adjust', 'Adjust', { isReady: (c) => c.films.length > 0 })];

const FRONT_FILM = caseAt('four-films-up').case.films.find((film) => film.kind === 'front');

let systemAsksForReduced = false;
const engines: { stop(): void }[] = [];

async function mountFrame(steps: readonly StepContract[], parts: RoomParts = NO_PARTS, engine?: EngineClient): Promise<void> {
  const running = engine ?? startStandInEngine();
  if (engine === undefined) engines.push(running as EngineClient & { stop(): void });
  putOnThePage(
    <EngineProvider engine={running}>
      <AppFrame steps={steps} parts={parts} />
    </EngineProvider>,
  );
  // The engine answers a moment after it is asked.
  await act(async () => {
    await running.info().catch(() => undefined);
  });
}

const root = () => find('.app-frame');

beforeEach(() => {
  systemAsksForReduced = false;
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => ({ matches: systemAsksForReduced, addEventListener: () => undefined, removeEventListener: () => undefined })),
  );
});

// Runs before the frame is taken down, so the stores are put back while the system's setting still answers.
afterEach(() => {
  act(() => {
    currentStep.getState().go('films');
    caseStore.getState().apply([{ kind: 'open-case', case: emptyCase() }]);
    caseSheetOpen.getState().setOpen(false);
    popoverOpen.getState().setOpen(false);
    activeTool.getState().pick(null);
    pointer.getState().leave();
    preferences.getState().setMotion('system', false);
    savedState.setState({ name: null, mark: null });
  });
  for (const engine of engines.splice(0)) engine.stop();
});

afterAll(() => {
  vi.unstubAllGlobals();
});

describe('the frame with no steps', () => {
  it('stands whole: top bar, an empty guide, an empty bench and the status line', async () => {
    await mountFrame([]);
    expect(find('.top-bar').textContent).toContain('Spine Workbench');
    expect(find('.case-title').textContent).toBe('Untitled caseNot saved yet');
    expect(find('.guide-column').children).toHaveLength(0);
    expect(find('.app-frame-bench').children).toHaveLength(0);
    expect(document.querySelectorAll('.step-rail-step')).toHaveLength(0);
    // The bench writes `data-lane` on the nearest element with `data-step`, so the attribute is there even now.
    expect(root().getAttribute('data-step')).toBe('');
  });

  it('shows the approved notice word for word, and a count of nothing sent', async () => {
    await mountFrame([]);
    expect(find('.status-line-claim').textContent).toBe(PREVIEW_NOTICE.short);
    expect(find('.status-line-sent').textContent).toBe('Requests sent: 0');
  });

  it('carries the engine’s state and version where the browser’s guards read them', async () => {
    const engine = startStandInEngine();
    engines.push(engine);
    putOnThePage(
      <EngineProvider engine={engine}>
        <AppFrame steps={[]} parts={NO_PARTS} />
      </EngineProvider>,
    );
    expect(find('[data-testid="engine-status"]').getAttribute('data-status')).toBe('starting');
    expect(document.querySelector('[data-testid="engine-version"]')).toBeNull();
    const info = await engine.info();
    await act(async () => {
      await Promise.resolve();
    });
    expect(find('[data-testid="engine-status"]').getAttribute('data-status')).toBe('ready');
    expect(find('[data-testid="engine-version"]').textContent).toBe(info.engineVersion);
  });

  it('says so in words, in the status line and once as a toast, when the engine does not start', async () => {
    const failing = { info: () => Promise.reject(new EngineFailure({ code: 'engine-load-failed', message: 'no engine', detail: null })) } as unknown as EngineClient;
    await mountFrame([], NO_PARTS, failing);
    expect(find('[data-testid="engine-status"]').getAttribute('data-status')).toBe('failed');
    const fault = find('.status-line-fault').textContent;
    expect(fault).toBe(faultWords('engine-load-failed').title);
    expect(saidNow()?.line).toContain(fault);
    expect(saidNow()?.tone).toBe('note');
  });
});

describe('the frame on steps 2 and 4', () => {
  it('puts the first step there is on screen, under the design’s short name', async () => {
    await mountFrame(TWO_STEPS);
    expect(root().getAttribute('data-step')).toBe('facing');
    expect(find('.guide-column h1').textContent).toBe('This is Size & facing.');
    expect([...document.querySelectorAll('.step-rail-step')].map((step) => [step.getAttribute('data-state'), step.querySelector('.step-rail-number')?.textContent])).toEqual([
      ['now', '2'],
      ['todo', '4'],
    ]);
    expect(find('[aria-current="step"]').textContent).toContain('Step 2, Size & facing');
  });

  it('follows the step on screen, and lets a step behind be pressed to go back', async () => {
    await mountFrame(TWO_STEPS);
    act(() => {
      currentStep.getState().go('adjust');
    });
    expect(root().getAttribute('data-step')).toBe('adjust');
    expect(find('.guide-column h1').textContent).toBe('This is Adjust.');
    const back = find('.step-rail-step button');
    expect(back.textContent).toContain('Step 2, Size & facing');
    pressOn(back);
    expect(currentStep.getState().slug).toBe('size-and-facing');
    expect(root().getAttribute('data-step')).toBe('facing');
  });

  it('stops in order: top bar, then the guide, then the bench', async () => {
    const Bench = () => <button type="button">A film slot</button>;
    await mountFrame(TWO_STEPS, { ...NO_PARTS, Bench });
    const stops = [...document.querySelectorAll<HTMLElement>('button:not(:disabled), [href], [tabindex]:not([tabindex="-1"])')];
    const place = (within: string) => stops.map((stop, at) => (stop.closest(within) === null ? -1 : at)).filter((at) => at !== -1);
    const [bar, guide, bench] = [place('.top-bar'), place('.guide-column'), place('.app-frame-bench')];
    expect(bar.length).toBeGreaterThan(0);
    expect(guide.length).toBeGreaterThan(0);
    expect(bench).toHaveLength(1);
    expect(Math.max(...bar)).toBeLessThan(Math.min(...guide));
    expect(Math.max(...guide)).toBeLessThan(Math.min(...bench));
    expect(bar.length + guide.length + bench.length).toBe(stops.length);
  });

  it('carries reduced motion and the tool in hand on its root', async () => {
    await mountFrame(TWO_STEPS);
    expect(root().getAttribute('data-motion')).toBe('full');
    expect(root().hasAttribute('data-tool')).toBe(false);
    act(() => {
      preferences.getState().setMotion('reduced', false);
      activeTool.getState().pick('boundary-wand');
    });
    expect(root().getAttribute('data-motion')).toBe('reduced');
    expect(root().getAttribute('data-tool')).toBe('boundary');
    act(() => {
      activeTool.getState().pick('pin');
    });
    expect(root().getAttribute('data-tool')).toBe('pins');
    act(() => {
      activeTool.getState().pick('ruler');
    });
    expect(root().hasAttribute('data-tool')).toBe(false);
  });

  it('is in reduced motion when the system asks, whatever the app’s own preference', async () => {
    systemAsksForReduced = true;
    await mountFrame(TWO_STEPS);
    expect(root().getAttribute('data-motion')).toBe('reduced');
  });
});

describe('the room’s parts', () => {
  it('mounts the bench and the sheet in the bench region, and stands the opening screen round the step', async () => {
    const Bench = () => <div className="fixture-bench" />;
    const CaseSheet = () => <div className="fixture-sheet" />;
    const OpeningCase = ({ children }: { children: ReactNode }) => <div className="fixture-opening">{children}</div>;
    await mountFrame(TWO_STEPS, { Bench, CaseSheet, OpeningCase });
    expect([...find('.app-frame-bench').children].map((child) => child.className)).toEqual(['fixture-bench', 'fixture-sheet']);
    expect(find('.guide-column .fixture-opening h1').textContent).toBe('This is Size & facing.');
  });

  it('gives the bench a root to write `data-lane` on', async () => {
    function Bench() {
      const here = useRef<HTMLDivElement>(null);
      useLayoutEffect(() => {
        here.current?.closest('[data-step]')?.setAttribute('data-lane', 'ladder');
      }, []);
      return <div ref={here} />;
    }
    await mountFrame([], { ...NO_PARTS, Bench });
    expect(root().getAttribute('data-lane')).toBe('ladder');
  });

  it('stands the guide’s lit button down while the case sheet is open', async () => {
    await mountFrame(TWO_STEPS);
    expect(find('.lit-button').hasAttribute('data-stood-down')).toBe(false);
    act(() => {
      caseSheetOpen.getState().setOpen(true);
    });
    expect(find('.lit-button').hasAttribute('data-stood-down')).toBe(true);
  });
});

describe('undo, redo and save', () => {
  const hangAFilm = () => {
    if (FRONT_FILM === undefined) throw new Error('the sample case has a front film');
    act(() => {
      caseStore.getState().apply([{ kind: 'set-film', film: FRONT_FILM }]);
    });
  };
  const films = () => caseStore.getState().case.films.length;
  const button = (name: string) => find(`.top-bar-actions button[aria-label="${name}"]`);

  it('answer Cmd or Ctrl+Z, with Shift for redo, in the Adjust step, and say what was done', async () => {
    await mountFrame(TWO_STEPS);
    hangAFilm();
    act(() => {
      currentStep.getState().go('adjust');
    });
    expect(pressKey('z', { ctrlKey: true }).defaultPrevented).toBe(true);
    expect(films()).toBe(0);
    expect(saidNow()?.line).toBe('Undid: hung a film.');
    expect(pressKey('Z', { metaKey: true, shiftKey: true }).defaultPrevented).toBe(true);
    expect(films()).toBe(1);
    expect(saidNow()?.line).toBe('Redid: hung a film.');
  });

  it('name the action in their tooltips, and are pressed like any button', async () => {
    await mountFrame(TWO_STEPS);
    hangAFilm();
    act(() => {
      currentStep.getState().go('adjust');
    });
    const tip = (name: string) => button(name).closest('.tooltip-anchor')?.querySelector('[role="tooltip"]')?.textContent;
    expect(tip('Undo')).toBe('Undo: Hung a film');
    expect(tip('Redo')).toBe('Nothing to redo');
    expect(button('Redo').getAttribute('aria-disabled')).toBe('true');
    pressOn(button('Undo'));
    expect(films()).toBe(0);
    expect(tip('Undo')).toBe('Nothing to undo');
    expect(tip('Redo')).toBe('Redo: Hung a film');
    pressOn(button('Redo'));
    expect(films()).toBe(1);
  });

  it('leave the case alone outside the Adjust step, and the key with whoever else wants it', async () => {
    await mountFrame(TWO_STEPS);
    hangAFilm();
    expect(pressKey('z', { metaKey: true }).defaultPrevented).toBe(false);
    expect(films()).toBe(1);
    expect(button('Undo').getAttribute('aria-disabled')).toBe('true');
    pressOn(button('Undo'));
    expect(films()).toBe(1);
  });

  it('open the case sheet on Cmd or Ctrl+S and from the button, once there is a case to save', async () => {
    await mountFrame(TWO_STEPS);
    // Nothing to save yet: the key is still taken, so the browser never offers to save the page.
    expect((button('Save case') as HTMLButtonElement).disabled).toBe(true);
    expect(pressKey('s', { metaKey: true }).defaultPrevented).toBe(true);
    expect(caseSheetOpen.getState().open).toBe(false);

    hangAFilm();
    expect((button('Save case') as HTMLButtonElement).disabled).toBe(false);
    pressKey('s', { ctrlKey: true });
    expect(caseSheetOpen.getState().open).toBe(true);

    act(() => {
      caseSheetOpen.getState().setOpen(false);
    });
    pressOn(button('Save case'));
    expect(caseSheetOpen.getState().open).toBe(true);
  });

  it('do not answer a plain Z or S', async () => {
    await mountFrame(TWO_STEPS);
    hangAFilm();
    act(() => {
      currentStep.getState().go('adjust');
    });
    expect(pressKey('z').defaultPrevented).toBe(false);
    expect(pressKey('s').defaultPrevented).toBe(false);
    expect(films()).toBe(1);
    expect(caseSheetOpen.getState().open).toBe(false);
  });
});

describe('the case, the pointer and the promise', () => {
  it('says the case holds unsaved work once a film is in it, and that it is saved once it is', async () => {
    await mountFrame(TWO_STEPS);
    const state = () => find('.case-title-state');
    act(() => {
      if (FRONT_FILM !== undefined) caseStore.getState().apply([{ kind: 'set-film', film: FRONT_FILM }]);
    });
    expect(state().textContent).toBe('Changes not saved');
    expect(state().hasAttribute('data-saved')).toBe(false);
    act(() => {
      savedState.getState().setName('spine-case-2026-11-12');
      savedState.getState().markSaved(caseStore.getState().version);
    });
    expect(find('.case-title').textContent).toBe('spine-case-2026-11-12Saved just now');
    expect(state().hasAttribute('data-saved')).toBe(true);
  });

  it('reads the pointer in pixels on a film with no size', async () => {
    await mountFrame(TWO_STEPS);
    act(() => {
      if (FRONT_FILM !== undefined) caseStore.getState().apply([{ kind: 'set-film', film: FRONT_FILM }]);
      pointer.getState().move({ film: 'front', at: [120.2, 340.7], screen: [0, 0] });
    });
    expect(find('.status-line-readings').textContent).toContain('x 120 px  y 341 px');
    act(() => {
      pointer.getState().leave();
    });
    expect(find('.status-line-readings').textContent).not.toContain('px');
  });

  it('opens the promise from the keyboard, and Escape closes it', async () => {
    await mountFrame(TWO_STEPS);
    const badge = find('.promise-badge');
    focusOn(badge);
    pressOn(badge);
    expect(find('[role="dialog"]').textContent).toContain('Nothing leaves this computer.');
    pressKey('Escape');
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.activeElement).toBe(badge);
  });
});
