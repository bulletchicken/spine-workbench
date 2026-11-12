// The design system's stylesheet reads the state of the page from five
// attributes on <html> (the head of components.css lists them). This is the
// one way the app writes them, so each has one writer and is taken away when
// that writer leaves the page.

import { useLayoutEffect } from 'react';

/** The attributes components.css reads on `<html>`, without their `data-`. */
export type PageStateName = 'moment' | 'films' | 'ready' | 'drag' | 'leaving';

/**
 * Sets `data-<name>` on `<html>` while the caller is mounted: a string as its
 * value, `true` as an empty attribute, and `false` or `null` as no attribute.
 * Set before the browser paints, so the stylesheet never draws a frame of the
 * state before.
 */
export function usePageState(name: PageStateName, value: string | boolean | null): void {
  useLayoutEffect(() => {
    const page = document.documentElement;
    const attribute = `data-${name}`;
    if (value === false || value === null) page.removeAttribute(attribute);
    else page.setAttribute(attribute, value === true ? '' : value);
    return () => {
      page.removeAttribute(attribute);
    };
  }, [name, value]);
}
