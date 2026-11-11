// Which figure stands for which film. A film kind with no figure does not
// compile, and a part that draws a film's figure looks it up here.

import type { ComponentType } from 'react';
import type { FilmKind } from '@/shapes';
import { BendLeftFigure } from './BendLeftFigure';
import { BendRightFigure } from './BendRightFigure';
import { FrontFigure } from './FrontFigure';
import { SideFigure } from './SideFigure';
import type { ViewFigureProps } from './ViewFigure';

export const FIGURE_FOR_VIEW: Record<FilmKind, ComponentType<ViewFigureProps>> = {
  front: FrontFigure,
  side: SideFigure,
  'bend-left': BendLeftFigure,
  'bend-right': BendRightFigure,
};
