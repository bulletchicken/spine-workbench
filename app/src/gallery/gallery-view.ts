// What the gallery is showing, kept in the address so that any view can
// be linked to, reloaded, or opened by a script:
//
//   ?gallery                          the first page, with the index
//   ?gallery=building-blocks/Button   that page
//   &width=1024                       in a 1024-wide frame, under the one breakpoint
//   &motion=reduced                   with reduced motion
//   &alone                            the page by itself, at the window's own size

import type { MotionMode } from '@/look-and-feel/data-attributes';

/** The two widths every part is compared at: above and below the one breakpoint. */
export const GALLERY_WIDTHS = [1440, 1024] as const;
export type GalleryWidth = (typeof GALLERY_WIDTHS)[number];

export interface GalleryView {
  /** A page's address, or `null` for the first page. */
  page: string | null;
  width: GalleryWidth;
  motion: MotionMode;
  /** The page with no index and no frame around it. */
  alone: boolean;
}

export function viewFrom(search: string): GalleryView {
  const asked = new URLSearchParams(search);
  const page = asked.get('gallery');
  return {
    page: page === null || page === '' ? null : page,
    width: asked.get('width') === '1024' ? 1024 : 1440,
    motion: asked.get('motion') === 'reduced' ? 'reduced' : 'full',
    alone: asked.has('alone'),
  };
}

/** The address of a view: what `viewFrom` reads back to the same view. */
export function searchFor(view: GalleryView): string {
  // Written by hand so a page's address keeps its slash, and the address stays readable.
  const parts = [view.page === null ? 'gallery' : `gallery=${encodeURI(view.page)}`];
  if (view.width !== 1440) parts.push(`width=${String(view.width)}`);
  if (view.motion !== 'full') parts.push(`motion=${view.motion}`);
  if (view.alone) parts.push('alone');
  return `?${parts.join('&')}`;
}
