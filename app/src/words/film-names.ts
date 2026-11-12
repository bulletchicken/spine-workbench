// Each film's name, as the screens say it.

import type { FilmKind } from '@/shapes';

export const FILM_NAMES: Record<FilmKind, string> = {
  front: 'Front',
  side: 'Side',
  'bend-left': 'Bending left',
  'bend-right': 'Bending right',
};
