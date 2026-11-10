// What a *.gallery.tsx file default-exports: one part of one area, shown
// in each of its states in the dev-only gallery at ?gallery.

import type { ReactNode } from 'react';

export interface GalleryPage {
  area: string;
  part: string;
  states: { name: string; show(): ReactNode }[];
}
