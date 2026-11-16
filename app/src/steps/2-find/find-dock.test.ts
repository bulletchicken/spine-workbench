import { describe, expect, it, vi } from 'vitest';
import { createFindReplay } from '@/auto-pass/find-replay-clock';
import { revealPlan } from '@/auto-pass/find-replay';
import { smallStory } from '@/engine-room/stand-in/small-story';
import type { ErrorCode, Level } from '@/shapes';
import { createFindState } from '@/view-state/find-state';
import { faultWords } from '@/words/faults';
import { FIND_WORDS } from '@/words/find';
import { findDock, type FindActions, type FindDockState } from './find-dock';

const act: FindActions = { review: vi.fn(), looksRight: vi.fn(), backToFilms: vi.fn(), tryAgain: vi.fn(), frontAlone: vi.fn() };
function state(): FindDockState {
  const films = smallStory().pass.marks;
  for (const film of films) for (const v of film.vertebrae) { v.confidence.band = 'high'; v.confidence.reasons = []; }
  return { find: createFindState().getState(), films, inPlace: 4, replay: createFindReplay().getState(), slowSearch: false, laterStep: null };
}
function doubtful(s: FindDockState, levels: readonly Level[]) {
  for (const film of s.films) for (const v of film.vertebrae) if (levels.includes(v.level)) v.confidence.band = 'low';
}

describe('the Find dock', () => {
  it('uses the singular, three-name and counted helper forms', () => {
    expect(FIND_WORDS.found(1)).toBe('1 vertebra found.');
    expect(FIND_WORDS.marked(['T1', 'T5', 'L5'], 4)).toBe('T1, T5 and L5 are marked on the films.');
    expect(FIND_WORDS.marked(['T1', 'T5', 'T9', 'L1', 'L5'], 1)).toBe('5 are marked on the film.');
  });
  it('N1 waits without a count; N2 counts only Front’s drawn outlines', () => {
    const s = state();
    s.find = { ...s.find, phase: 'searching', share: 0.45, searched: 4 };
    expect(findDock(s, act)).toEqual({ scene: 'finding', sentence: 'Finding the spine…', helper: { kind: 'meter', share: 0.45, words: 'Searching 4 films.' } });
    // Only the made-up case's four films take long enough to say how long.
    s.slowSearch = true;
    expect(findDock(s, act).helper).toMatchObject({ words: 'Searching 4 films. About 15 seconds.' });
    s.find.searched = 1;
    expect(findDock(s, act).helper).toMatchObject({ words: 'Searching the Front film.' });
    s.slowSearch = false;
    const plan = revealPlan(s.films, {}, { height: 200, edge: 60, filmHeight: 1300 });
    if (plan === null) throw new Error('no plan');
    s.find.phase = 'replaying';
    s.replay = { ...s.replay, plan, time: 2900, times: { lead: 700, travel: 4400, hold: 560 } };
    expect(findDock(s, act).helper).toMatchObject({ kind: 'count', count: '2 of 3', spoken: '2 of 3', share: 2 / 3 });
  });
  it('N3 and N6 name the levels needing a look, with a quiet review and no Start over', () => {
    const s = state(); s.find.phase = 'found'; doubtful(s, ['L3', 'L5']);
    expect(findDock(s, act)).toEqual({ scene: 'found', sentence: '3 vertebrae found. 2 need a look.', helper: { kind: 'look', words: 'L3 and L5 are marked on the films.' }, quiet: { label: 'Review the 2', press: act.review } });
    s.find.looked = ['L3']; s.inPlace = 1;
    expect(findDock(s, act)).toMatchObject({ sentence: '3 vertebrae found. 1 needs a look.', helper: { words: 'L5 is marked on the film.' }, quiet: { label: 'Review it' } });
  });
  it('N4 says all look right, and N5 keeps the estimated-names words after review', () => {
    const s = state(); s.find.phase = 'found';
    expect(findDock(s, act)).toEqual({ scene: 'found', sentence: '3 vertebrae found.', helper: { kind: 'words', words: ['All of them look right.'] } });
    const front = s.films.find((film) => film.film === 'front');
    if (front === undefined) throw new Error('no Front');
    front.naming = { ...front.naming, flags: ['estimated'] };
    doubtful(s, ['L5']);
    expect(findDock(s, act).helper).toEqual({ kind: 'look', words: 'Names are estimates. Check where T1 is.' });
    s.find.looked = ['L5'];
    expect(findDock(s, act)).toMatchObject({ sentence: '3 vertebrae found.', helper: { kind: 'words', words: ['Names are estimates. Check where T1 is.'] } });
  });
  it('N7 and N8 keep one Looks right action, and say which films to check', () => {
    const s = state(); s.find.phase = 'found'; doubtful(s, ['L3', 'L5']);
    s.find.reviewing = ['L3', 'L5']; s.find.review = 0;
    expect(findDock(s, act)).toMatchObject({ scene: 'review', sentence: 'L3 needs a look.', helper: { count: '1 of 2', words: 'Check its outline on both films.' }, primary: { label: 'Looks right', press: act.looksRight } });
    s.find.review = 1;
    expect(findDock(s, act).helper).toMatchObject({ count: '2 of 2' });
    const one = { ...s, films: s.films.filter((film) => film.film === 'front') };
    expect(findDock(one, act).helper).toMatchObject({ words: 'Check its outline on the film.' });
    const v = one.films[0]?.vertebrae[2];
    if (v?.marks.kind !== 'front') throw new Error('no outline');
    v.marks.points.supL.inferred = true;
    expect(findDock(one, act).helper).toMatchObject({ words: 'Filled in from its neighbours. Check it on the film.' });
    expect(findDock(s, act).quiet).toBeUndefined();
  });
  it.each([1, 4])('N9 and N10 offer the way back, or Front alone, after %i films', (searched) => {
    const s = state(); s.find = { ...s.find, phase: 'failed', searched, failure: 'pass-no-spine-found' };
    expect(findDock(s, act)).toEqual(searched === 1 ? {
      scene: 'failed', sentence: 'No spine was found on the Front film.', helper: { kind: 'words', words: ['Check it is a spine X-ray and the right way up.'] }, primary: { label: 'Back to the films', press: act.backToFilms },
    } : {
      scene: 'failed', sentence: 'One film showed no spine.', helper: { kind: 'words', words: ['Search Front alone, or go back.'] }, quiet: { label: 'Back to the films', press: act.backToFilms }, primary: { label: 'Search Front alone', press: act.frontAlone },
    });
  });
  it.each<ErrorCode>(['pass-film-too-small', 'pass-flat-film', 'pass-too-little-usable', 'pass-strip-too-short'])('N11 words %s for one film and several', (code) => {
    const s = state(); s.find = { ...s.find, phase: 'failed', searched: 1, failure: code };
    expect(findDock(s, act)).toMatchObject({ sentence: 'The Front film could not be searched.', helper: { words: [`${faultWords(code).title}.`] }, primary: { label: 'Back to the films' } });
    s.find.searched = 4;
    expect(findDock(s, act)).toMatchObject({ sentence: 'One film could not be searched.', primary: { label: 'Search Front alone' }, quiet: { label: 'Back to the films' } });
  });
  it.each<ErrorCode>(['engine-crashed', 'engine-out-of-memory', 'engine-out-of-sync'])('N12 words %s, with a retry', (code) => {
    const s = state(); s.find = { ...s.find, phase: 'failed', searched: 4, failure: code };
    expect(findDock(s, act)).toEqual({ scene: 'failed', sentence: 'The search stopped before it finished.', helper: { kind: 'words', words: [`${faultWords(code).title}.`] }, quiet: { label: 'Back to the films', press: act.backToFilms }, primary: { label: 'Try again', press: act.tryAgain } });
  });
  it('offers Build the 3D spine exactly when a later step is registered, disabled until found', () => {
    const s = state(); const press = vi.fn();
    for (const phase of ['searching', 'replaying', 'found'] as const) {
      s.find.phase = phase;
      expect(findDock(s, act).primary).toBeUndefined();
      const withStep = findDock({ ...s, laterStep: { label: '3D', press } }, act);
      expect(withStep.primary).toEqual({ label: 'Build the 3D spine', disabled: phase !== 'found', press });
    }
    findDock({ ...s, laterStep: { label: '3D', press } }, act).primary?.press();
    expect(press).toHaveBeenCalledOnce();
  });
  it('keeps Build the 3D spine disabled, and says why, while the films give fewer than two vertebrae', () => {
    const s = state(); const press = vi.fn();
    s.find.phase = 'found';
    const front = s.films.find((film) => film.film === 'front'), side = s.films.find((film) => film.film === 'side');
    if (front === undefined || side === undefined) throw new Error('the story has no front and side');
    const keep = (film: typeof front, levels: readonly Level[]) => { film.vertebrae = film.vertebrae.filter((v) => levels.includes(v.level)); };
    keep(front, ['L4']); keep(side, []);
    const one = findDock({ ...s, laterStep: { label: '3D', press } }, act);
    expect(one).toMatchObject({ sentence: '1 vertebra found.', helper: { kind: 'words', words: ['The 3D spine needs at least two vertebrae.'] }, primary: { label: 'Build the 3D spine', disabled: true } });
    keep(side, ['L4']);
    expect(findDock({ ...s, laterStep: { label: '3D', press } }, act).primary?.disabled).toBe(true);
    // A Side with two of its own gives the fit its second level.
    side.vertebrae = smallStory().pass.marks.find((film) => film.film === 'side')?.vertebrae.filter((v) => v.level === 'L4' || v.level === 'L5') ?? [];
    expect(side.vertebrae).toHaveLength(2);
    expect(findDock({ ...s, laterStep: { label: '3D', press } }, act).primary?.disabled).toBe(false);
    front.vertebrae = smallStory().pass.marks.find((film) => film.film === 'front')?.vertebrae.filter((v) => v.level === 'L4' || v.level === 'L5') ?? []; keep(side, []);
    const two = findDock({ ...s, laterStep: { label: '3D', press } }, act);
    expect(two.primary?.disabled).toBe(false); expect(two.helper).not.toMatchObject({ words: ['The 3D spine needs at least two vertebrae.'] });
  });
});
