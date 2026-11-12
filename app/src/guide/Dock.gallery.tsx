import type { GalleryPage } from '@/gallery/gallery-page';
import type { ErrorCode } from '@/shapes';
import { faultWords } from '@/words/faults';
import { FILMS_WORDS as Films } from '@/words/films';
import { FIND_WORDS as Find } from '@/words/find';
import { SHELL_WORDS } from '@/words/shell';
import { Dock } from './Dock';
import type { DockAction, DockHelper, DockSpec } from './dock-spec';
import './Dock.gallery.css';

const nothing = () => undefined;
const action = (label: string): DockAction => ({ label, press: nothing });
const words = (text: string): DockHelper => ({ kind: 'words', words: [text] });
const count = (at: number, total: number, text?: string): DockHelper => ({
  kind: 'count', count: Films.readCount(at, total), spoken: Films.readCount(at, total), share: at / total,
  ...(text === undefined ? {} : { words: text }),
});
const primary = (disabled = false) => ({ ...action(Films.findTheSpine), disabled });
const add: DockSpec = {
  scene: 'add', sentence: Films.add,
  helper: { kind: 'words', words: [Films.addHelp[0], { link: Films.addHelp[1], press: nothing } satisfies { link: string; press: () => void }, Films.addHelp[2]] },
  quiet: action(Films.useSampleCase), primary: primary(true),
};
const ready = (sentence: string, helper = Films.canFind, disabled = false): DockSpec => ({
  scene: 'ready', sentence, helper: words(helper), quiet: action(Films.startOver), primary: primary(disabled),
});
const fourIn = ready(Films.allIn, Films.allInHelp);
const found: DockSpec = {
  scene: 'found', sentence: Find.found(17), helper: words(Find.allLookRight),
};
const needsLook: DockSpec = {
  ...found, sentence: `${Find.found(17)} ${Find.needALook(2)}`,
  helper: { kind: 'look', words: Find.marked(['T1', 'L5'], 4) }, quiet: action(Find.review(2)),
};
const review = (level: 'T1' | 'L5', at: number, helper = Find.checkOutline): DockSpec => ({
  scene: 'review', sentence: Find.needsALook(level), helper: count(at, 2, helper), primary: action(Find.looksRight),
});
const stopped = (code: ErrorCode): DockSpec => ({
  scene: 'failed', sentence: Find.stopped, helper: words(Find.faultHelp(faultWords(code).title)),
  quiet: action(Find.backToFilms), primary: action(Find.tryAgain),
});
const unread = (n: number, helper: string, base = add): DockSpec => ({ ...base, sentence: Films.refused(n), helper: words(helper) });
const sample = (sentence: string, next: string): DockSpec => ({
  scene: 'ready', sentence, helper: words(Films.sampleHelp), quiet: action(next), primary: primary(),
});
const noSpine = (alone: boolean): DockSpec => ({
  scene: 'failed', sentence: alone ? Find.noSpineFront : Find.noSpineSome,
  helper: words(alone ? Find.noSpineFrontHelp : Find.noSpineSomeHelp),
  ...(alone ? {} : { quiet: action(Find.backToFilms) }),
  primary: action(alone ? Find.backToFilms : Find.frontAlone),
});

const show = (spec: DockSpec | null) => () => <div className="dock-gallery"><Dock spec={spec} /></div>;
const state = (name: string, spec: DockSpec | null) => ({ name, show: show(spec) });

const page: GalleryPage = {
  area: 'guide', part: 'Dock', states: [
    state('No step', null),
    state('Empty, the made-up sample next', add),
    state('Empty, a sample film next', { ...add, quiet: action(Films.useSampleFilm) }),
    state('Files held, the dock keeps its words', add),
    state('Reading one film', { scene: 'reading', sentence: Films.readingOne, helper: words(Films.goesIn('front')), primary: primary(true) }),
    state('Reading several films', { scene: 'reading', sentence: Films.readingMany, helper: count(2, 4), primary: primary(true) }),
    state('Drawing the made-up case', { scene: 'reading', sentence: Films.gettingSampleCase, helper: words(Films.sampleCaseHelp), primary: primary(true) }),
    state('Reading the made-up case', { scene: 'reading', sentence: Films.gettingSampleCase, helper: count(2, 4), primary: primary(true) }),
    state('Front only', ready(Films.frontIn)),
    state('Two films in', ready(Films.someIn(2))),
    state('Three films in', ready(Films.someIn(3))),
    state('Four films in, including the made-up case', fourIn),
    state('Front still to add', ready(Films.frontToAdd, Films.frontToAddHelp, true)),
    state('A sample film, another follows', sample(Films.sampleIn(2, 6), Films.nextSampleFilm)),
    state('The last sample film', sample(Films.sampleIn(6, 6), Films.useSampleCase)),
    state('The only sample film', sample(Films.oneSampleIn, Films.useSampleCase)),
    state('One unread file, empty stage', unread(1, Films.refusedHelp(faultWords('film-unknown-format').title))),
    state('One unread file, earlier films stay', unread(1, Films.refusedHelp(faultWords('film-truncated').title), fourIn)),
    state('A saved case given as a film', unread(1, Films.savedCase)),
    state('Several unread files, some read', unread(2, Films.othersAreIn, ready(Films.frontIn))),
    state('No file could be read', unread(2, Films.refusedHelp(faultWords('film-truncated').title))),
    state('One file left out', { ...fourIn, sentence: Films.leftOut(1), helper: words(Films.leftOutHelp) }),
    state('Several files left out', { ...fourIn, sentence: Films.leftOut(2), helper: words(Films.leftOutHelp) }),
    state('A film changed during the search', { ...fourIn, sentence: Films.searchStopped, helper: words(Films.searchStoppedHelp) }),
    state('Start over, the dock waits for the stage', fourIn),
    state('Searching Front alone', { scene: 'finding', sentence: Find.finding, helper: { kind: 'meter', share: 0.6, words: Find.searching(1) } }),
    state('Searching several films', { scene: 'finding', sentence: Find.finding, helper: { kind: 'meter', share: 0.6, words: Find.searching(3) } }),
    state('Replaying the answer', { scene: 'finding', sentence: Find.finding, helper: count(9, 17) }),
    state('Two levels need a look', needsLook),
    state('One level on one film needs a look', { ...needsLook, sentence: `${Find.found(17)} ${Find.needALook(1)}`, helper: { kind: 'look', words: Find.marked(['L5'], 1) }, quiet: action(Find.review(1)) }),
    state('Three named levels need a look', { ...needsLook, sentence: `${Find.found(17)} ${Find.needALook(3)}`, helper: { kind: 'look', words: Find.marked(['T1', 'T5', 'L5'], 4) }, quiet: action(Find.review(3)) }),
    state('Five levels need a look', { ...needsLook, sentence: `${Find.found(17)} ${Find.needALook(5)}`, helper: { kind: 'look', words: Find.marked(['T1', 'T5', 'T9', 'L1', 'L5'], 4) }, quiet: action(Find.review(5)) }),
    state('All levels looked at', found),
    state('Estimated names, some need a look', { ...needsLook, helper: { kind: 'look', words: Find.estimated } }),
    state('Estimated names, after review', { ...found, helper: words(Find.estimated) }),
    state('Pointing at an outline keeps the dock words', needsLook),
    state('Review on both standing films', review('T1', 1)),
    state('Review on one standing film', review('T1', 1, Find.checkOutlineOne)),
    state('A filled-in outline', review('T1', 1, Find.filledIn)),
    state('The last level in the review', review('L5', 2)),
    state('No spine on Front alone', noSpine(true)),
    state('No spine on one of several films', noSpine(false)),
    ...(['pass-film-too-small', 'pass-flat-film', 'pass-too-little-usable', 'pass-strip-too-short'] as const).flatMap((code) => [true, false].map((alone) => state(`${code}, ${alone ? 'Front alone' : 'several films'}`, {
      ...noSpine(alone), sentence: alone ? Find.notSearchedFront : Find.notSearchedSome,
      helper: words(Find.faultHelp(faultWords(code).title)),
    }))),
    ...(['engine-crashed', 'engine-out-of-memory', 'engine-out-of-sync'] as const).map((code) => state(`Search stopped: ${faultWords(code).title}`, stopped(code))),
    ...(['engine-load-failed', 'engine-out-of-sync'] as const).map((code) => state(`Engine did not start: ${faultWords(code).title}`, {
      scene: 'engine', sentence: `${faultWords(code).title}.`, helper: words(SHELL_WORDS.reload),
    })),
    state('Engine still starting, films can be given', add),
  ],
};
export default page;
