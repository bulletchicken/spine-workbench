import { exclusionWords, type SideExclusion } from '@/bench/search-films';
import { SIDE_WORDS } from '@/words/side';
import type { DockHelper, DockSpec } from '@/guide/dock-spec';
import type { FilmKind } from '@/shapes';
import type { FilmIntakeState } from '@/view-state/film-intake';
import { faultWords } from '@/words/faults';
import { FILMS_WORDS as W } from '@/words/films';

export interface FilmsState { inPlace: readonly FilmKind[]; intake: FilmIntakeState; sampleFilms: number; canFind?: boolean; sideExclusion?: SideExclusion | null }
export interface FilmsActions { choose: () => void; sample: () => void; startOver: () => void; find: () => void }
const words = (text: string): DockHelper => ({ kind: 'words', words: [text] });

export function filmsDock({ inPlace, intake, sampleFilms, canFind = inPlace.includes('front'), sideExclusion = null }: FilmsState, act: FilmsActions): DockSpec {
  const primary = { label: W.findTheSpine, press: act.find, disabled: !canFind };
  const sampleLabel = intake.nextSample >= sampleFilms ? W.useSampleCase : W.useSampleFilm;
  let spec: DockSpec;
  if (intake.making || intake.reading.length > 0) {
    const sample = intake.making || intake.from === 'made-up-case';
    const count = W.readCount(intake.readDone, intake.readTotal);
    spec = {
      scene: 'reading',
      sentence: sample ? W.gettingSampleCase : intake.readTotal === 1 ? W.readingOne : W.readingMany,
      helper: intake.making ? words(W.sampleCaseHelp) : intake.readTotal === 1
        ? words(W.goesIn(intake.reading[0] ?? 'front'))
        : { kind: 'count', count, spoken: count, share: intake.readDone / intake.readTotal },
      primary: { ...primary, disabled: true },
    };
    return spec;
  }
  if (inPlace.length === 0) {
    spec = {
      scene: 'add', sentence: W.add,
      helper: { kind: 'words', words: [W.addHelp[0], { link: W.addHelp[1], press: act.choose } satisfies { link: string; press: () => void }, W.addHelp[2]] } satisfies DockHelper,
      quiet: { label: sampleLabel, press: act.sample }, primary,
    };
  } else {
    const sample = intake.from === 'sample-film';
    const front = inPlace.includes('front');
    spec = {
      scene: 'ready',
      sentence: sample ? sampleFilms === 1 ? W.oneSampleIn : W.sampleIn((intake.sample ?? 0) + 1, sampleFilms)
        : !front && inPlace.includes('side') ? SIDE_WORDS.sideIn : !front ? W.frontToAdd : inPlace.length === 4 ? W.allIn : inPlace.length === 1 ? W.frontIn : W.someIn(inPlace.length),
      helper: words(sample ? W.sampleHelp : !front ? W.frontToAddHelp : inPlace.length === 4 ? W.allInHelp : W.canFind),
      quiet: { label: sample ? intake.nextSample >= sampleFilms ? W.useSampleCase : W.nextSampleFilm : W.startOver, press: sample ? act.sample : act.startOver },
      primary,
    };
  }
  if (sideExclusion !== null) spec = { ...spec, helper: words(exclusionWords(sideExclusion)) };
  else if (canFind && !inPlace.includes('front') && inPlace.includes('side')) spec = { ...spec, helper: words(W.canFind) };
  const notice = intake.notice;
  if (notice === null) return spec;
  if (notice.kind === 'left-out') return { ...spec, sentence: W.leftOut(notice.count), helper: words(W.leftOutHelp) };
  if (notice.kind === 'search-stopped') return { ...spec, sentence: W.searchStopped, helper: words(W.searchStoppedHelp) };
  const title = notice.code === 'a-saved-case' ? W.savedCase : W.refusedHelp(faultWords(notice.code).title);
  return { ...spec, sentence: W.refused(notice.count), helper: words(notice.count > 1 && intake.readDone > 0 ? W.othersAreIn : title) };
}
