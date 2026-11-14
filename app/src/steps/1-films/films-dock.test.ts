import { describe, expect, it, vi } from 'vitest';
import { createFilmIntake, type IntakeNotice } from '@/view-state/film-intake';
import { FILM_KINDS } from '@/case-state/selectors';
import type { FilmKind } from '@/shapes';
import type { DockHelper } from '@/guide/dock-spec';
import type { FilmIntakeState } from '@/view-state/film-intake';
import { filmsDock, type FilmsActions } from './films-dock';

const act: FilmsActions = { choose: vi.fn(), sample: vi.fn(), startOver: vi.fn(), find: vi.fn() };
function dock(inPlace: readonly FilmKind[] = [], patch: Partial<FilmIntakeState> = {}, sampleFilms = 0) {
  return filmsDock({ inPlace, intake: { ...createFilmIntake().getState(), ...patch }, sampleFilms }, act);
}

describe('the Films dock', () => {
  it('F1 and F2: adds films through one link and the next sample, with Find disabled', () => {
    expect(dock()).toEqual({ scene: 'add', sentence: 'Add the four films.', helper: { kind: 'words', words: ['Drop them anywhere, or ', { link: 'choose them', press: act.choose } satisfies { link: string; press: () => void }, '.'] } satisfies DockHelper, quiet: { label: 'Use the sample case', press: act.sample }, primary: { label: 'Find the spine', press: act.find, disabled: true } });
    expect(dock([], {}, 6).quiet?.label).toBe('Use a sample film');
    expect(dock([], { nextSample: 6 }, 6).quiet?.label).toBe('Use the sample case');
  });
  it('F3: reads one into its named place, with no quiet action', () => {
    expect(dock([], { reading: ['side'], readTotal: 1 })).toEqual({ scene: 'reading', sentence: 'Reading the film…', helper: { kind: 'words', words: ['It goes in the Side place.'] }, primary: { label: 'Find the spine', press: act.find, disabled: true } });
  });
  it('F4: counts the films already read outside the spoken line', () => {
    expect(dock(['front', 'side'], { reading: ['bend-left', 'bend-right'], readTotal: 4, readDone: 2 })).toEqual({ scene: 'reading', sentence: 'Reading the films…', helper: { kind: 'count', count: '2 of 4', spoken: '2 of 4', share: 0.5 }, primary: { label: 'Find the spine', press: act.find, disabled: true } });
  });
  it('F5: drawing precedes reading the made-up case', () => {
    const drawing = dock([], { making: true });
    expect(drawing.sentence).toBe('Getting the sample case ready…');
    expect(drawing.helper).toEqual({ kind: 'words', words: ['Four made-up films, drawn by the app.'] });
    expect(drawing.quiet).toBeUndefined();
    expect(drawing.primary?.disabled).toBe(true);
    const reading = dock(['front'], { from: 'made-up-case', reading: ['side', 'bend-left', 'bend-right'], readTotal: 4, readDone: 1 });
    expect(reading.sentence).toBe(drawing.sentence);
    expect(reading.helper).toEqual({ kind: 'count', count: '1 of 4', spoken: '1 of 4', share: 0.25 });
  });
  it.each([
    [['front'], 'The Front film is in.', 'You can find the spine now, or add the others.', false],
    [['front', 'side'], 'Two films are in.', 'You can find the spine now, or add the others.', false],
    [['front', 'side', 'bend-left'], 'Three films are in.', 'You can find the spine now, or add the others.', false],
    [FILM_KINDS, 'All four films are in.', 'Each one is in its place.', false],
    [['side'], 'The Side film is in.', 'The search starts from the Front film.', true],
  ] as const)('F6 to F9: words and actions for %j', (films, sentence, helper, disabled) => {
    expect(dock(films)).toEqual({ scene: 'ready', sentence, helper: { kind: 'words', words: [helper] }, quiet: { label: 'Start over', press: act.startOver }, primary: { label: 'Find the spine', press: act.find, disabled } });
  });
  it('F8 and F11: the made-up case uses the same four-film words', () => {
    expect(dock(FILM_KINDS, { from: 'made-up-case' })).toEqual(dock(FILM_KINDS, { from: 'yours' }));
  });
  it('F10: numbers samples, and the last one leads to the made-up case', () => {
    expect(dock(['front'], { from: 'sample-film', sample: 1, nextSample: 2 }, 6)).toEqual({ scene: 'ready', sentence: 'Sample film 2 of 6 is in.', helper: { kind: 'words', words: ['Drop your own films at any time.'] }, quiet: { label: 'Next sample film', press: act.sample }, primary: { label: 'Find the spine', press: act.find, disabled: false } });
    expect(dock(['front'], { from: 'sample-film', sample: 0, nextSample: 1 }, 1)).toMatchObject({ sentence: 'The sample film is in.', quiet: { label: 'Use the sample case' } });
    expect(dock(['front'], { from: 'sample-film', sample: 5, nextSample: 6 }, 6).quiet?.label).toBe('Use the sample case');
  });
  it.each([
    [{ kind: 'refused', count: 1, code: 'film-unknown-format' }, [], 'One file could not be read.', 'This file is not a film the app can read.'],
    [{ kind: 'refused', count: 1, code: 'a-saved-case' }, [], 'One file could not be read.', 'This is a saved case, not a film.'],
    [{ kind: 'refused', count: 2, code: 'film-truncated' }, ['front'], 'Two files could not be read.', 'The others are in their places.'],
    [{ kind: 'refused', count: 10, code: 'film-truncated' }, [], '10 files could not be read.', 'This file ends too soon.'],
    [{ kind: 'left-out', count: 1 }, FILM_KINDS, 'One file was left out.', 'Every place is full. Drop on a film to replace it.'],
    [{ kind: 'left-out', count: 2 }, FILM_KINDS, 'Two files were left out.', 'Every place is full. Drop on a film to replace it.'],
    [{ kind: 'search-stopped' }, ['front'], 'The search was stopped.', 'A film changed. Find the spine again when ready.'],
  ] satisfies [IntakeNotice, FilmKind[] | readonly FilmKind[], string, string][])('F12 to F15: overlays %j and keeps the underlying actions', (notice, films, sentence, helper) => {
    expect(dock(films, { notice, readDone: helper === 'The others are in their places.' ? 1 : 0 })).toEqual({ ...dock(films), sentence, helper: { kind: 'words', words: [helper] } });
  });
  it('F16: keeps its words and actions through the leaving hold', () => {
    expect(dock(['front'], { leaving: true })).toEqual(dock(['front']));
  });
});
