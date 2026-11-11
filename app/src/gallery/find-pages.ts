// Finds every gallery page by its file name, so no slice edits a shared
// list: a `*.gallery.tsx` file anywhere under src/ is in the gallery.

import type { GalleryPage } from './gallery-page';

export interface FoundPage {
  /** The page's address in the gallery: its path under src/ without the ending, such as `building-blocks/Button`. */
  id: string;
  page: GalleryPage;
}

/** The pages of the files found, each under its address, in the order the index lists them: by area, then by part. */
export function pagesFrom(found: Readonly<Record<string, GalleryPage>>): FoundPage[] {
  return Object.entries(found)
    .map(([path, page]) => ({ id: path.replace(/^(\.\.\/)+/, '').replace(/\.gallery\.tsx$/, ''), page }))
    .sort((a, b) => a.page.area.localeCompare(b.page.area) || a.page.part.localeCompare(b.page.part) || a.id.localeCompare(b.id));
}

/** Every gallery page in the app. */
export function findPages(): FoundPage[] {
  return pagesFrom(import.meta.glob<GalleryPage>('../**/*.gallery.tsx', { eager: true, import: 'default' }));
}
