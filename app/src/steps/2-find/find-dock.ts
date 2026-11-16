import { SIDE_WORDS as S } from '@/words/side';
import { shownAt } from '@/auto-pass/find-replay';
import type { FindReplayClock } from '@/auto-pass/find-replay-clock';
import type { DockAction, DockSpec } from '@/guide/dock-spec';
import { lookLevels } from '@/review/look-levels';
import { compareLevels, type FilmMarks } from '@/shapes';
import type { FindState } from '@/view-state/find-state';
import { faultWords } from '@/words/faults';
import { FIND_WORDS as W } from '@/words/find';

export interface FindDockState {
  trainedSide?: boolean;
  find: FindState;
  films: readonly FilmMarks[];
  inPlace: number;
  replay: ReturnType<FindReplayClock['getState']>;
  /** The made-up case's four films go through the hand-built search, which takes long enough to say so. */
  slowSearch: boolean;
  laterStep: DockAction | null;
}
export interface FindActions { review: () => void; looksRight: () => void; backToFilms: () => void; tryAgain: () => void; frontAlone: () => void }

export function findDock(state: FindDockState, act: FindActions): DockSpec {
  const { find, films, replay, trainedSide = false } = state;
  const count = films.find((film) => film.film === 'front')?.vertebrae.filter((v) => compareLevels(v.level, 'T1') >= 0).length ?? 0;
  const side = films.find((f) => f.film === 'side');
  const sideOnly = count === 0 && (side?.vertebrae.length ?? 0) > 0;
  // The fit needs two levels. A Side brings its own only when it has two of them.
  const tooFew = count > 0 && (films.find((film) => film.film === 'front')?.vertebrae.length ?? 0) < 2 && (side?.vertebrae.length ?? 0) < 2;
  const searchedSideOnly = find.kinds.length === 1 && find.kinds[0] === 'side';
  const needed = lookLevels(films.filter((f) => !trainedSide || f.film !== 'side')).filter((entry) => !find.looked.includes(entry.level));
  const level = find.review === null ? undefined : find.reviewing[find.review];
  if (level !== undefined) {
    const entry = lookLevels(films.filter((f) => !trainedSide || f.film !== 'side')).find((entry) => entry.level === level);
    const both = entry?.on.front !== undefined && entry.on.side !== undefined;
    return {
      scene: 'review', sentence: W.needsALook(level),
      helper: { kind: 'count', count: W.reviewCount((find.review ?? 0) + 1, find.reviewing.length), spoken: W.reviewCount((find.review ?? 0) + 1, find.reviewing.length), share: ((find.review ?? 0) + 1) / find.reviewing.length, words: entry?.why === 'filled-in' ? W.filledIn : both ? W.checkOutline : W.checkOutlineOne },
      primary: { label: W.looksRight, press: act.looksRight },
    };
  }
  if (find.phase === 'failed') {
    const code = find.failure ?? 'engine-crashed';
    const alone = find.searched === 1;
    const noSpine = code === 'pass-no-spine-found';
    const filmFault = noSpine || ['pass-film-too-small', 'pass-flat-film', 'pass-too-little-usable', 'pass-strip-too-short'].includes(code);
    return {
      scene: 'failed',
      sentence: filmFault ? noSpine ? alone ? searchedSideOnly ? S.noSpine : W.noSpineFront : W.noSpineSome : alone ? searchedSideOnly ? S.notSearched : W.notSearchedFront : W.notSearchedSome : W.stopped,
      helper: { kind: 'words', words: [noSpine ? alone ? W.noSpineFrontHelp : W.noSpineSomeHelp : W.faultHelp(faultWords(code).title)] },
      ...(filmFault && alone ? {} : { quiet: { label: W.backToFilms, press: act.backToFilms } }),
      primary: filmFault ? alone ? { label: W.backToFilms, press: act.backToFilms } : { label: W.frontAlone, press: act.frontAlone } : { label: W.tryAgain, press: act.tryAgain },
    };
  }
  // The registered destination controls both the waiting and the finished dock.
  const next = state.laterStep === null ? {} : { primary: { ...state.laterStep, label: W.buildTheSpine, disabled: find.phase !== 'found' || count === 0 || tooFew } };
  if (find.phase !== 'found') {
    const total = replay.plan?.counted.length ?? 0;
    const shown = replay.plan === null ? 0 : shownAt(replay.plan, replay.times, replay.time).count;
    return {
      scene: 'finding', sentence: W.finding,
      helper: find.phase === 'replaying'
        ? { kind: 'count', count: W.count(shown, total), spoken: W.count(shown, total), share: total === 0 ? 0 : shown / total }
        : { kind: 'meter', share: find.share, words: state.slowSearch && find.searched > 1 ? `${W.searching(find.searched)} ${W.aboutFifteenSeconds}` : searchedSideOnly ? S.searching : W.searching(find.searched) },
      ...next,
    };
  }
  const estimated = films.find((film) => film.film === 'front')?.naming.flags.includes('estimated') ?? false;
  const words = estimated ? W.estimated : needed.length > 0 ? W.marked(needed.map((entry) => entry.level), state.inPlace) : W.allLookRight;
  return {
    scene: 'found', sentence: sideOnly ? S.found : `${W.found(count)}${needed.length > 0 ? ` ${W.needALook(needed.length)}` : ''}`,
    helper: sideOnly ? { kind: 'words', words: [S.noFront] } : tooFew ? { kind: 'words', words: [W.needTwo] } : !trainedSide ? needed.length > 0 ? { kind: 'look', words } : { kind: 'words', words: [words] } : { kind: 'words', words: [`${words} ${S.found}`] },
    ...(needed.length > 0 ? { quiet: { label: W.review(needed.length), press: act.review } } : {}),
    ...next,
  };
}
