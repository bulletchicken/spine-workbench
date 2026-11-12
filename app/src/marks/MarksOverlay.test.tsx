// The outlines layer, drawn through a real layer slot and layer stack, as
// a film tile draws it. What is checked is the two passes the design
// system's stylesheet lays out, the classes it reads, what a screen reader
// is handed, and what the pointer reports.

import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { find, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { filmSpace } from '@/film-viewer/film-space';
import { LayerStack } from '@/film-viewer/LayerStack';
import { useLayer } from '@/film-viewer/layer-slot';
import type { MarkToDraw, Quad } from './mark-to-draw';
import { MarksOverlay, type MarksOverlayProps } from './MarksOverlay';

const space = filmSpace({ size: { width: 400, height: 800 }, view: { quarterTurns: 0, mirrored: false }, zoom: 2, pan: [0, 0] });

const quadAt = (top: number): Quad => [
  [100, top],
  [180, top + 4],
  [176, top + 50],
  [102, top + 46],
];

function vertebra(id: number, name: string, top: number, more: Partial<MarkToDraw> = {}): MarkToDraw {
  return { key: String(id), name, quad: quadAt(top), look: 'found', shown: true, spoken: name, ...more };
}

const three = (): MarkToDraw[] => [
  vertebra(1, 'T1', 100),
  vertebra(2, 'T2', 160, { look: 'look', spoken: 'T2, needs a look' }),
  vertebra(3, 'T3', 220, { shown: false }),
];

function Drawn(props: Partial<MarksOverlayProps>) {
  const layer = useLayer('outlines', MarksOverlay, { label: 'Outlines on the Front film', marks: three(), on: null, only: null, ...props });
  return <LayerStack space={space} layers={[layer]} />;
}

const groups = () => [...find('.marks').children] as SVGGElement[];
const under = () => [...document.querySelectorAll<SVGGElement>('.mark.under')];
const lines = () => [...document.querySelectorAll<SVGGElement>('.mark:not(.under)')];
const line = (name: string) => find(`.mark:not(.under)[aria-label^="${name}"]`);
const states = (each: SVGGElement) => ['found', 'look', 'on', 'focus'].filter((name) => each.classList.contains(name));

function point(on: Element, type: 'pointerover' | 'pointerout') {
  act(() => {
    on.dispatchEvent(new MouseEvent(type, { bubbles: true }));
  });
}

describe('the outlines layer', () => {
  it('draws two passes: every fill and casing first, then every line and name', () => {
    putOnThePage(<Drawn />);
    expect(groups().map((group) => group.getAttribute('class')?.startsWith('mark under') ?? false)).toEqual([true, true, true, false, false, false]);
    for (const group of groups()) {
      const polygon = find('polygon', group);
      expect(polygon.getAttribute('pathLength')).toBe('1');
    }
    expect(find('.mark.under polygon').getAttribute('points')).toBe('100,100 180,104 176,150 102,146');
    expect(find('polygon', line('T1')).getAttribute('points')).toBe('100,100 180,104 176,150 102,146');
  });

  it('puts the same state classes on both groups of a vertebra', () => {
    const page = putOnThePage(<Drawn />);
    expect(under().map(states)).toEqual([['found'], ['found', 'look'], []]);
    expect(lines().map(states)).toEqual(under().map(states));

    page.show(<Drawn on="1" only="2" />);
    expect(lines().map(states)).toEqual([['found', 'on'], ['found', 'look', 'focus'], []]);
    expect(under().map(states)).toEqual(lines().map(states));
  });

  it('is spoken as the film’s outlines, each by its name, leaving out casings and an outline not yet shown', () => {
    putOnThePage(<Drawn />);
    const layer = find('.marks');
    expect(layer.getAttribute('role')).toBe('group');
    expect(layer.getAttribute('aria-label')).toBe('Outlines on the Front film');
    expect(under().every((group) => group.getAttribute('aria-hidden') === 'true')).toBe(true);
    expect(lines().map((group) => [group.getAttribute('role'), group.getAttribute('aria-label'), group.getAttribute('aria-hidden')])).toEqual([
      ['group', 'T1', null],
      ['group', 'T2, needs a look', null],
      ['group', 'T3', 'true'],
    ]);
  });

  it('names each outline at its right-most corner and the middle of its height, in film pixels', () => {
    putOnThePage(<Drawn />);
    const name = find('text', line('T1'));
    expect(name.textContent).toBe('T1');
    expect(name.getAttribute('x')).toBe('180');
    expect(name.getAttribute('y')).toBe('125');
    // The name is always drawn: the tile's data-names hides it on the small films.
    expect(document.querySelectorAll('.mark text')).toHaveLength(3);
  });

  it('reports the pointer coming onto a shown outline and leaving it', () => {
    const pointed: (string | null)[] = [];
    putOnThePage(<Drawn onPoint={(key) => pointed.push(key)} />);
    point(find('polygon', line('T2')), 'pointerover');
    point(find('polygon', line('T2')), 'pointerout');
    expect(pointed).toEqual(['2', null]);
    // One not shown yet is not pointed at.
    point(find('polygon', line('T3')), 'pointerover');
    expect(pointed).toEqual(['2', null]);
  });

  it('takes no pointer while a review is on', () => {
    const pointed: (string | null)[] = [];
    putOnThePage(<Drawn only="2" onPoint={(key) => pointed.push(key)} />);
    point(find('polygon', line('T1')), 'pointerover');
    point(find('polygon', line('T2')), 'pointerover');
    expect(pointed).toEqual([]);
  });

  it('holds nothing that takes focus', () => {
    putOnThePage(<Drawn onPoint={() => undefined} />);
    expect(document.querySelectorAll('.marks [tabindex], .marks a, .marks button')).toHaveLength(0);
  });

  it('names no colour, width or time: the stylesheet is the design system’s', () => {
    putOnThePage(<Drawn on="1" />);
    const drawn = document.querySelectorAll('.marks, .marks *');
    for (const each of drawn) {
      for (const attribute of ['style', 'fill', 'stroke', 'stroke-width', 'opacity']) expect(each.hasAttribute(attribute), `${each.tagName} ${attribute}`).toBe(false);
    }
  });
});
