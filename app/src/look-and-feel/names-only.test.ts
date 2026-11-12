// The design system's rule for its own stylesheet, held for the app's: a
// rule uses the names in tokens.css and writes no colour, length or time as
// a number (DESIGN.md, "Keeping this current"). The stylesheets of the
// earlier look are let by while they are still in the tree; a part that is
// restyled or deleted comes off the list, and the list only shrinks.

import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

/** The stylesheets of the earlier look, by path under app/src. */
const STILL_EARLIER = [
  'app-shell/CaseTitle.css',
  'app-shell/GuideColumn.css',
  'app-shell/StatusLine.css',
  'app-shell/StepRail.css',
  'building-blocks/Dialog.css',
  'building-blocks/IconButton.css',
  'building-blocks/Kbd.css',
  'building-blocks/Slider.css',
  'building-blocks/Tooltip.css',
  'building-blocks/toast/Toast.css',
  'film-viewer/FilmViewer.css',
  'film-viewer/LayerStack.css',
  'gallery/Gallery.css',
  'gallery/GalleryStage.css',
  'guide/ChoiceCards.css',
  'guide/Guide.css',
  'guide/HalfFilmFigure.css',
  'guide/QuietActions.css',
  'guide/cards/CardRows.css',
  'guide/cards/ChecksCard.css',
  'guide/cards/GuideCard.css',
  'guide/cards/HowSureCard.css',
  'guide/cards/NarrationCard.css',
  'look-and-feel/ConfidenceGlyph.css',
  'look-and-feel/StateIcon.css',
  'look-and-feel/confidence.css',
  'look-and-feel/figures/view-figure.css',
  'look-and-feel/icons/icon.css',
  'spine-3d/view/gallery-parts.css',
];

/**
 * Sheets that hold values rather than use them, or are scaffolding: the
 * tokens, the earlier tokens and timings, the faces, the
 * choreography's, and the gallery's own pages.
 */
const NOT_HELD = /^look-and-feel\/(tokens|earlier-tokens|named-timings|fonts)\.css$|^choreography\/[^/]+\.css$|\.gallery\.css$/;

/** The search DESIGN.md runs over components.css, and the two things it lets by. */
const WRITTEN_AS_A_NUMBER = /#[0-9a-fA-F]{3,8}\b|rgba?\(|[0-9.]+(px|ms|em|rem|vw|vh)\b/;
const LET_BY = /\* 1px|100v[wh]/;

const source = fileURLToPath(new URL('..', import.meta.url));
const read = (path: string) => readFileSync(join(source, path), 'utf8');

function stylesheets(folder: string): string[] {
  return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const path = join(folder, entry.name);
    if (entry.isDirectory()) return stylesheets(path);
    return entry.name.endsWith('.css') ? [relative(source, path)] : [];
  });
}

/** A stylesheet as CSS reads it: comments out, and the conditions of its media queries, which are not values. */
function rulesOf(css: string): string {
  return css.replace(/\/\*[\s\S]*?\*\//g, '').replace(/@media[^{]*\{/g, '@media {');
}

const namesSetIn = (css: string) => new Set([...css.matchAll(/(--[a-z0-9-]+)\s*:/g)].map(([, name = '']) => name));

describe('the app’s own stylesheets', () => {
  const held = stylesheets(source)
    .filter((path) => !NOT_HELD.test(path) && !STILL_EARLIER.includes(path))
    .sort();

  it('list only stylesheets of the earlier look that still exist', () => {
    const all = stylesheets(source);
    for (const path of STILL_EARLIER) expect(all, path).toContain(path);
  });

  it('really hold the system’s stylesheet, the base sheet and the outlines’', () => {
    expect(held).toEqual(expect.arrayContaining(['look-and-feel/components.css', 'look-and-feel/base.css', 'marks/marks.css', 'building-blocks/Button.css']));
  });

  it('write no colour, length or time as a number: they use the names', () => {
    const found: string[] = [];
    for (const path of held) {
      // The one rule let by, by name: the clip that keeps spoken words off the screen is not a design value.
      const css = rulesOf(read(path)).replace(/\.spoken-only\s*\{[^}]*\}/g, '');
      for (const line of css.split('\n')) {
        if (WRITTEN_AS_A_NUMBER.test(line) && !LET_BY.test(line)) found.push(`${path}: ${line.trim()}`);
      }
    }
    expect(found).toEqual([]);
  });

  it('read none of the earlier look’s tokens', () => {
    // The two inks the earlier film hand-over still sets are the system's names on the page, so they are not earlier names.
    const system = namesSetIn(read('look-and-feel/tokens.css'));
    const earlier = new Set([...namesSetIn(read('look-and-feel/earlier-tokens.css'))].filter((name) => !system.has(name)));
    const found: string[] = [];
    for (const path of held) {
      for (const [, name = ''] of rulesOf(read(path)).matchAll(/var\(\s*(--[a-z0-9-]+)/g)) {
        if (earlier.has(name)) found.push(`${path}: ${name}`);
      }
    }
    expect(found).toEqual([]);
  });
});
