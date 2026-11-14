import type { FilmKind } from '@/shapes';
import { FILM_NAMES } from './film-names';
export const THREE_WORDS = {
  done: 'Done', railSlot: 'Put the film back in the rail',
  railFilm: (kind: FilmKind, selected: boolean) => `${FILM_NAMES[kind]} film. ${selected ? 'Shown beside the spine.' : 'Show it beside the spine.'}`,
  comparison: (kind: FilmKind) => `${FILM_NAMES[kind]} film beside the spine.`,
  compareHelper: 'Point at a vertebra to see it on both.', bendHelper: 'Shown beside the standing spine.',
  stepName: '3D',
  building: 'Building the 3D spine…',
  failed: 'The 3D spine did not finish building.',
  retry: 'Try again',
  settled: 'The spine in 3D.',
  pair: 'Built from the front and side films. Drag to turn it.',
  front: 'Built from the front film. Its depth and side curve are typical values, not measured.',
  unusedSide: 'The side film could not be used.',
  partial: 'Some vertebrae were not found, so they are not shown.',
  view: 'View',
  frontView: 'Front', sideView: 'Side', topView: 'Top',
  bonesReady: '3D drawing ready.',
};
export const ASSUMED_FACING_NOTE = 'Left and right assume the film is shown the usual way round.';
