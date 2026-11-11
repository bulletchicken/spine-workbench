// The hooks in components: the case and its checks, read by two parts at
// once, from the app's own stores.

import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CaseFileV1, CheckFinding } from '@/shapes';
import type { EngineClient } from '@/engine-room/engine-client';
import { caseStore, useCase } from './case-store';
import { sketchFilm } from './sketch-case';
import { useCaseChecks } from './use-case-checks';
import { CHECK_DELAY_MS } from './watch-case-checks';

declare global {
  var IS_REACT_ACT_ENVIRONMENT: boolean | undefined;
}

const finding: CheckFinding = { kind: 'l5-unconfirmed', severity: 'warning', film: 'front', vertebraId: null, level: null };

/** An engine that finds one thing per film in the case it is given. */
const checkCase = vi.fn((c: CaseFileV1) => Promise.resolve(c.films.map(() => finding)));
const engine = { checkCase } as unknown as EngineClient;

function Findings({ name }: { name: string }) {
  const version = useCase((store) => store.version);
  const { findings, checkedAt } = useCaseChecks(engine);
  return (
    <p data-part={name}>
      {`version ${String(version)}; ${findings === null ? 'not checked' : `${String(findings.length)} found at ${String(checkedAt)}`}`}
    </p>
  );
}

let root: Root;
let host: HTMLElement;

const shown = (name: string) => host.querySelector(`[data-part="${name}"]`)?.textContent;

beforeEach(() => {
  globalThis.IS_REACT_ACT_ENVIRONMENT = true;
  vi.useFakeTimers();
  host = document.createElement('div');
  root = createRoot(host);
});

afterEach(() => {
  act(() => {
    root.unmount();
  });
  vi.useRealTimers();
});

describe('useCase and useCaseChecks', () => {
  it('show the case and its findings in every part that reads them, asking the engine once per change', async () => {
    act(() => {
      root.render(
        <>
          <Findings name="one" />
          <Findings name="two" />
        </>,
      );
    });
    expect(shown('one')).toBe('version 0; not checked');
    await act(() => vi.advanceTimersByTimeAsync(CHECK_DELAY_MS));
    expect(checkCase).toHaveBeenCalledTimes(1);
    expect(shown('two')).toBe('version 0; 0 found at 0');

    act(() => {
      caseStore.getState().apply([{ kind: 'set-film', film: sketchFilm('side') }]);
    });
    expect(shown('one')).toBe('version 1; 0 found at 0');
    await act(() => vi.advanceTimersByTimeAsync(CHECK_DELAY_MS));
    expect(checkCase).toHaveBeenCalledTimes(2);
    expect([shown('one'), shown('two')]).toEqual(['version 1; 1 found at 1', 'version 1; 1 found at 1']);
  });

  it('stop asking once no part reads the checks', async () => {
    const calls = checkCase.mock.calls.length;
    act(() => {
      root.render(<Findings name="one" />);
    });
    act(() => {
      root.render(<p />);
    });
    const version = caseStore.getState().version;
    act(() => {
      caseStore.getState().apply([{ kind: 'set-film', film: sketchFilm('bend-left') }]);
    });
    expect(caseStore.getState().version).toBe(version + 1);
    await act(() => vi.advanceTimersByTimeAsync(CHECK_DELAY_MS * 2));
    expect(checkCase.mock.calls.length).toBe(calls);
  });

  it('keep the findings when one screen replaces another and the case has not changed', async () => {
    act(() => {
      root.render(
        <section>
          <Findings name="one" />
        </section>,
      );
    });
    await act(() => vi.advanceTimersByTimeAsync(CHECK_DELAY_MS));
    const version = caseStore.getState().version;
    const found = `version ${String(version)}; ${String(caseStore.getState().case.films.length)} found at ${String(version)}`;
    expect(shown('one')).toBe(found);
    act(() => {
      root.render(
        <article>
          <Findings name="two" />
        </article>,
      );
    });
    expect(shown('one')).toBeUndefined();
    expect(shown('two')).toBe(found);
  });

  it('show a part mounted after the watch stopped no findings for a version the case has left', async () => {
    act(() => {
      root.render(<Findings name="one" />);
    });
    await act(() => vi.advanceTimersByTimeAsync(CHECK_DELAY_MS));
    const version = caseStore.getState().version;
    expect(shown('one')).toMatch(new RegExp(`found at ${String(version)}$`));
    act(() => {
      root.render(<p />);
    });
    act(() => {
      caseStore.getState().apply([{ kind: 'set-film', film: sketchFilm('bend-right') }]);
    });
    act(() => {
      root.render(<Findings name="two" />);
    });
    expect(shown('two')).toBe(`version ${String(version + 1)}; not checked`);
    await act(() => vi.advanceTimersByTimeAsync(CHECK_DELAY_MS));
    expect(shown('two')).toBe(`version ${String(version + 1)}; ${String(caseStore.getState().case.films.length)} found at ${String(version + 1)}`);
  });
});
