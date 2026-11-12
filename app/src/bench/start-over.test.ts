import { afterEach, describe, expect, it, vi } from 'vitest';
import type { FilmKind } from '@/shapes';
import { caseStore } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { FILM_KINDS } from '@/case-state/selectors';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { filmRecordFrom } from '@/open-and-save/film-record';
import { currentStep } from '@/view-state/current-step';
import { filmIntake } from '@/view-state/film-intake';
import { selection } from '@/view-state/selection';
import { popoverOpen } from '@/on-this-device/popover-open';
import { startOver } from './start-over';

vi.mock('@/choreography/timing', () => ({ durationOnPage: () => 260 }));
afterEach(() => { vi.useRealTimers(); });

describe('starting over', () => {
  it('closes the note, holds the old stage for the fade, then forgets every film and keeps the next sample', async () => {
    const retireFilm = vi.fn<(kind: FilmKind) => void>();
    const story = smallStory();
    caseStore.getState().apply([{ kind: 'set-film', film: filmRecordFrom(story.films.front.facts) }]);
    filmPixels.getState().opened('front', { pixels: new Uint16Array([1]), thumbnail: new Uint8Array() });
    filmIntake.getState().next(3);
    selection.getState().select({ film: 'front', vertebra: 1 });
    selection.getState().hover({ film: 'front', vertebra: 1 });
    popoverOpen.getState().setOpen(true);
    currentStep.getState().go('find');
    vi.useFakeTimers();
    const leaving = startOver({ retireFilm });
    await vi.advanceTimersByTimeAsync(0);
    expect(filmIntake.getState().leaving).toBe(true);
    expect(popoverOpen.getState().open).toBe(false);
    await vi.advanceTimersByTimeAsync(259);
    expect(caseStore.getState().case.films).toHaveLength(1);
    expect(retireFilm).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    await leaving;
    expect(retireFilm.mock.calls.map(([kind]) => kind)).toEqual(FILM_KINDS);
    expect(caseStore.getState().case.films).toEqual([]);
    expect(filmPixels.getState().films).toEqual({});
    expect(filmIntake.getState()).toMatchObject({ leaving: false, nextSample: 3, from: null });
    expect(selection.getState()).toMatchObject({ selected: null, hot: null });
    expect(currentStep.getState().slug).toBe('films');
  });
});
