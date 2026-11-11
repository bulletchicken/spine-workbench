// Nothing in the app rests on colour alone, so every sign that carries a
// state must be its own shape. These tests draw each sign with no
// stylesheet at all and compare the drawings: two states that drew the
// same shapes could only be told apart by hue.

import type { ReactElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { ConfidenceGlyph, ConfidenceGlyphShapes } from './ConfidenceGlyph';
import type { Conf, RowState } from './data-attributes';
import { BendLeftFigure } from './figures/BendLeftFigure';
import { BendRightFigure } from './figures/BendRightFigure';
import { FrontFigure } from './figures/FrontFigure';
import { SideFigure } from './figures/SideFigure';
import { Icon } from './icons/Icon';
import { DRAWN_ICON_NAMES, LUCIDE_ICON_NAMES } from './icons/icon-name';
import { StateIcon } from './StateIcon';

const CONFS: readonly Conf[] = ['sure', 'look', 'needs', 'byyou', 'sacrum'];
const ROW_STATES: readonly RowState[] = ['ok', 'needs', 'warn', 'bad', 'you', 'wait', 'busy'];

/** What is drawn inside the outermost element, whatever attributes that element carries. */
function drawing(element: ReactElement): string {
  return renderToStaticMarkup(element).replace(/^<[^>]*>/, '').replace(/<\/[^>]*>$/, '');
}

function outer(element: ReactElement): string {
  return /^<[^>]*>/.exec(renderToStaticMarkup(element))?.[0] ?? '';
}

describe('the five confidence glyphs', () => {
  it('are five different shapes', () => {
    expect(new Set(CONFS.map((conf) => drawing(<ConfidenceGlyph conf={conf} />))).size).toBe(5);
  });

  it('each carry their state for CSS and are hidden from assistive technology', () => {
    for (const conf of CONFS) {
      const svg = outer(<ConfidenceGlyph conf={conf} />);
      expect(svg).toContain(`data-conf="${conf}"`);
      expect(svg).toContain('aria-hidden="true"');
    }
  });

  it('draw the same shapes inside another drawing, with a casing under them when asked', () => {
    for (const conf of CONFS) {
      const alone = renderToStaticMarkup(<ConfidenceGlyphShapes conf={conf} />);
      const cased = renderToStaticMarkup(<ConfidenceGlyphShapes conf={conf} cased />);
      expect(alone).toBe(drawing(<ConfidenceGlyph conf={conf} />));
      expect(cased.endsWith(alone)).toBe(true);
      expect(cased.slice(0, -alone.length)).toContain('confidence-glyph-casing');
    }
  });

  // The two glyphs with a hollow, measured by hand from their rings: a ring
  // of radius r drawn with a line w wide reaches out to r + w/2.
  //
  //   look    r 3.7,  w 1.5   reaches 4.45
  //   needs   r 3.45, w 2.1   reaches 4.5
  //
  // Their casing is one filled disc, at least a unit wider than the ring, so
  // the film never shows through the hollow.
  it('case a glyph that has a hollow with a whole disc, wider than its ring', () => {
    for (const [conf, reach] of [
      ['look', 4.45],
      ['needs', 4.5],
    ] as const) {
      const alone = renderToStaticMarkup(<ConfidenceGlyphShapes conf={conf} />);
      const casing = renderToStaticMarkup(<ConfidenceGlyphShapes conf={conf} cased />).slice(0, -alone.length);
      const disc = /^<circle class="confidence-glyph-casing-disc" r="([\d.]+)"><\/circle>$/.exec(casing);
      expect(disc, conf).not.toBeNull();
      expect(Number(disc?.[1]), conf).toBeGreaterThanOrEqual(reach + 1);
    }
  });
});

describe('the state icons', () => {
  it('are six different shapes for seven states: a chip that needs an answer warns as a row does', () => {
    const drawings = new Map(ROW_STATES.map((state) => [state, drawing(<StateIcon state={state} />)]));
    expect(drawings.get('needs')).toBe(drawings.get('warn'));
    expect(new Set(drawings.values()).size).toBe(6);
  });

  it('each carry their state for CSS and are hidden from assistive technology', () => {
    for (const state of ROW_STATES) {
      const svg = outer(<StateIcon state={state} />);
      expect(svg).toContain(`data-state="${state}"`);
      expect(svg).toContain('aria-hidden="true"');
    }
  });
});

describe('the icons', () => {
  const names = [...DRAWN_ICON_NAMES, ...LUCIDE_ICON_NAMES];

  it('number eight drawn for this app and seventeen from Lucide, with no name used twice', () => {
    expect(DRAWN_ICON_NAMES).toHaveLength(8);
    expect(LUCIDE_ICON_NAMES).toHaveLength(17);
    expect(new Set(names).size).toBe(25);
  });

  it('each draw something of their own', () => {
    const drawings = names.map((name) => drawing(<Icon name={name} />));
    for (const drawn of drawings) expect(drawn).not.toBe('');
    expect(new Set(drawings).size).toBe(names.length);
  });

  it('are all found by one class, named for CSS, and hidden from assistive technology', () => {
    for (const name of names) {
      const svg = outer(<Icon name={name} />);
      expect(svg, name).toMatch(/class="[^"]*\bicon\b/);
      expect(svg, name).toContain(`data-icon="${name}"`);
      expect(svg, name).toContain('aria-hidden="true"');
    }
  });

  it('sit on a grid of 20 when drawn here and 24 when Lucide’s', () => {
    for (const name of DRAWN_ICON_NAMES) expect(outer(<Icon name={name} />), name).toContain('viewBox="0 0 20 20"');
    for (const name of LUCIDE_ICON_NAMES) expect(outer(<Icon name={name} />), name).toContain('viewBox="0 0 24 24"');
  });

  it('keep a class the part gives them', () => {
    expect(outer(<Icon name="pin" className="tool-icon" />)).toContain('class="icon tool-icon"');
    expect(outer(<Icon name="check" className="tool-icon" />)).toMatch(/class="[^"]*\bicon tool-icon\b/);
  });
});

describe('the four view figures', () => {
  it('are four different drawings on one 60 by 120 grid', () => {
    const figures = [<FrontFigure key="front" />, <SideFigure key="side" />, <BendLeftFigure key="left" />, <BendRightFigure key="right" />];
    expect(new Set(figures.map(drawing)).size).toBe(4);
    for (const figure of figures) expect(outer(figure)).toContain('viewBox="0 0 60 120"');
  });

  it('lean right as the mirror of leaning left', () => {
    const left = drawing(<BendLeftFigure />);
    expect(drawing(<BendRightFigure />)).toBe(`<g transform="matrix(-1 0 0 1 60 0)">${left}</g>`);
  });

  it('each have one spine, cut into whole dashes', () => {
    for (const figure of [<FrontFigure key="front" />, <SideFigure key="side" />, <BendLeftFigure key="left" />]) {
      expect(drawing(figure).match(/class="view-figure-spine" pathLength="45"/g)).toHaveLength(1);
    }
  });
});
