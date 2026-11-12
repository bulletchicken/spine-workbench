import type { Level } from '@/shapes';

export const FIND_WORDS = {
  stepName: 'Find',
  finding: 'Finding the spine…',
  count: (shown: number, total: number) => `${String(shown)} of ${String(total)}`,
  buildTheSpine: 'Build the 3D spine',
  found: (n: number) => `${String(n)} ${n === 1 ? 'vertebra' : 'vertebrae'} found.`,
  needALook: (n: number) => `${String(n)} ${n === 1 ? 'needs' : 'need'} a look.`,
  marked: (levels: readonly Level[], films: number) => {
    const names = levels.length >= 4 ? String(levels.length) : levels.length < 2 ? levels.join('') : `${levels.slice(0, -1).join(', ')} and ${levels.at(-1) ?? ''}`;
    return `${names} ${levels.length === 1 ? 'is' : 'are'} marked on the ${films === 1 ? 'film' : 'films'}.`;
  },
  allLookRight: 'All of them look right.',
  review: (n: number) => n === 1 ? 'Review it' : `Review the ${String(n)}`,
  needsALook: (level: Level) => `${level} needs a look.`,
  reviewCount: (at: number, of: number) => `${String(at)} of ${String(of)}`,
  checkOutline: 'Check its outline on both films.',
  checkOutlineOne: 'Check its outline on the film.',
  filledIn: 'Filled in from its neighbours. Check it on the film.',
  looksRight: 'Looks right',
  searching: (n: number) => n === 1 ? 'Searching the Front film.' : `Searching ${String(n)} films.`,
  aboutFifteenSeconds: 'About 15 seconds.',
  estimated: 'Names are estimates. Check where T1 is.',
  backToFilms: 'Back to the films',
  tryAgain: 'Try again',
  frontAlone: 'Search Front alone',
  noSpineFront: 'No spine was found on the Front film.',
  noSpineFrontHelp: 'Check it is a spine X-ray and the right way up.',
  noSpineSome: 'One film showed no spine.',
  noSpineSomeHelp: 'Search Front alone, or go back.',
  notSearchedFront: 'The Front film could not be searched.',
  notSearchedSome: 'One film could not be searched.',
  stopped: 'The search stopped before it finished.',
  faultHelp: (title: string) => `${title}.`,
};
