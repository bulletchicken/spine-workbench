import type { FilmKind } from '@/shapes';
import { FILM_NAMES } from './film-names';

const COUNTS = ['Zero', 'One', 'Two', 'Three', 'Four', 'Five', 'Six', 'Seven', 'Eight', 'Nine'];
const countWord = (n: number): string => COUNTS[n] ?? String(n);

export const FILMS_WORDS = {
  stepName: 'Films',
  add: 'Add the four films.',
  addHelp: ['Drop them anywhere, or ', 'choose them', '.'] as const,
  useRealSampleCase: 'Use the real sample case',
  gettingRealSampleCase: 'Getting the real sample case ready…',
  realSampleCaseHelp: 'Real films of one patient, kept on this machine.',
  credit: (credit: string) => `Films: ${credit}`,
  useSampleCase: 'Use the sample case',
  useSampleFilm: 'Use a sample film',
  nextSampleFilm: 'Next sample film',
  findTheSpine: 'Find the spine',
  startOver: 'Start over',
  readingOne: 'Reading the film…',
  readingMany: 'Reading the films…',
  goesIn: (kind: FilmKind) => `It goes in the ${FILM_NAMES[kind]} place.`,
  readCount: (done: number, total: number) => `${String(done)} of ${String(total)}`,
  gettingSampleCase: 'Getting the sample case ready…',
  sampleCaseHelp: 'Four made-up films, drawn by the app.',
  frontIn: 'The Front film is in.',
  someIn: (n: number) => `${countWord(n)} films are in.`,
  allIn: 'All four films are in.',
  allInHelp: 'Each one is in its place.',
  canFind: 'You can find the spine now, or add the others.',
  frontToAdd: 'The Front film is still to add.',
  frontToAddHelp: 'The search starts from the Front film.',
  sampleIn: (k: number, n: number) => `Sample film ${String(k)} of ${String(n)} is in.`,
  oneSampleIn: 'The sample film is in.',
  sampleHelp: 'Drop your own films at any time.',
  refused: (n: number) => `${countWord(n)} ${n === 1 ? 'file' : 'files'} could not be read.`,
  refusedHelp: (title: string) => `${title}.`,
  savedCase: 'This is a saved case, not a film.',
  othersAreIn: 'The others are in their places.',
  leftOut: (n: number) => `${countWord(n)} ${n === 1 ? 'file was' : 'files were'} left out.`,
  leftOutHelp: 'Every place is full. Drop on a film to replace it.',
  searchStopped: 'The search was stopped.',
  searchStoppedHelp: 'A film changed. Find the spine again when ready.',
};
