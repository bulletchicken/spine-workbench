import { Fragment, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import type { FilmKind } from '@/shapes';
import { durationOf } from '@/choreography/timing';
import type { FilmSize } from '@/film-viewer/film-space';
import { usePageState } from '@/look-and-feel/page-state';
import type { MarkToDraw } from '@/marks/mark-to-draw';
import type { ReplaySignal } from '@/marks/TravellingLight';
import { FILM_NAMES } from '@/words/film-names';
import { STAGE_WORDS } from '@/words/stage';
import type { FilmFrame } from './film-framing';
import { FilmPlaces } from './FilmPlaces';
import { FilmTile } from './FilmTile';
import { flightFrom, fly, readLayout } from './fly-tiles';
import { motionIsReduced, useGlidingCameras } from './glide-cameras';
import { CAPTION, captionSpot, FILM_ORDER, stageLayout, type Spot, type StageLayout, RAIL_HEIGHT, RAIL_GAP } from './stage-layout';

export type Moment = 'films' | 'find' | 'three';
export interface StageFilm {
  size: FilmSize;
  pixels: Uint16Array;
  thumbnail: Uint8Array;
  outlines: readonly MarkToDraw[];
  names: boolean;
  on: string | null;
  only: string | null;
  frame: FilmFrame;
  light: { from: number; to: number; height: number } | null;
}
export interface Held { over: boolean; target: FilmKind | null }
export interface StageProps {
  moment: Moment;
  films: Readonly<Record<FilmKind, StageFilm | null>>;
  reading: readonly FilmKind[];
  held: Held;
  leaving: boolean;
  marked: boolean;
  reviewing: boolean;
  replay: ReplaySignal | null;
  onPressPlace?: (kind: FilmKind) => void;
  onPoint?: (film: FilmKind, key: string | null) => void;
}

interface Measures { width: number; height: number; displayHeight: number; railHeight: number; railGap: number; dx: number; dy: number }
interface Drawing {
  layout: StageLayout;
  places: Record<FilmKind, Spot>;
  moment: Moment;
  present: string;
  measures: Measures;
}

/** Only the page's fractional offset is removed; the pure layout stays in whole stage pixels. */
function onPixels(layout: StageLayout, { dx, dy }: Measures): StageLayout {
  return { displayTop: layout.displayTop - dy, spots: Object.fromEntries(FILM_ORDER.map((kind) => {
    const spot = layout.spots[kind];
    return [kind, spot === null ? null : { ...spot, x: spot.x - dx, y: spot.y - dy }];
  })) as StageLayout['spots'] };
}

function placeSpots(layout: StageLayout): Record<FilmKind, Spot> {
  return Object.fromEntries(FILM_ORDER.map((kind) => {
    const spot = layout.spots[kind];
    if (spot === null) throw new Error('a film place has no spot');
    return [kind, spot];
  })) as Record<FilmKind, Spot>;
}

/** Writes a complete resting layout, also used while a reduced-motion fade holds the old one. */
function draw(tiles: HTMLElement, drawing: Drawing): void {
  const stage = tiles.parentElement;
  if (stage === null) return;
  const display = stage.querySelector<HTMLElement>('.display');
  display?.style.setProperty('--head-y', String(drawing.layout.displayTop));
  const zone = stage.querySelector<HTMLElement>('.dropzone');
  const first = drawing.places.front;
  const last = drawing.places['bend-right'];
  for (const [name, value] of Object.entries({ '--zx': first.x, '--zy': first.y, '--zw': last.x + last.width - first.x, '--zh': first.height + CAPTION, '--tw': first.width, '--th': first.height })) zone?.style.setProperty(name, String(value));
  for (const kind of FILM_ORDER) {
    const spot = drawing.layout.spots[kind] ?? drawing.places[kind];
    const tile = tiles.querySelector<HTMLElement>(`.tile[data-place="${kind}"]`);
    for (const [name, value] of Object.entries({ '--x': spot.x, '--y': spot.y, '--bw': spot.width, '--bh': spot.height, '--s': 1 })) tile?.style.setProperty(name, String(value));
    tile?.style.setProperty('--r', `var(--r-${spot.radius})`);
    stage.querySelector<HTMLElement>(`.place[data-place="${kind}"]`)?.style.setProperty('--px', String(drawing.places[kind].x - first.x));
  }
  drawCaptions(tiles, drawing);
}

function drawCaptions(tiles: HTMLElement, drawing: Drawing): void {
  for (const kind of FILM_ORDER) {
    const at = captionSpot(drawing.layout.spots[kind] ?? drawing.places[kind]);
    const caption = tiles.querySelector<HTMLElement>(`.caption[data-place="${kind}"]`);
    caption?.style.setProperty('--cx', String(at.centreX));
    caption?.style.setProperty('--cy', String(at.top));
  }
}

export function Stage({ moment, films, reading, held, leaving, marked, reviewing, replay, onPressPlace, onPoint }: StageProps) {
  const tiles = useRef<HTMLDivElement>(null);
  const display = useRef<HTMLHeadingElement>(null);
  const [measures, setMeasures] = useState<Measures>({ width: 0, height: 0, displayHeight: 0, railHeight: RAIL_HEIGHT, railGap: RAIL_GAP, dx: 0, dy: 0 });
  const [fontsReady, setFontsReady] = useState(false);
  const previous = useRef<Drawing | null>(null);
  useGlidingCameras(tiles);

  useLayoutEffect(() => {
    const stage = tiles.current?.parentElement;
    if (stage === undefined || stage === null) return undefined;
    let mounted = true;
    const measure = (width: number, height: number) => {
      const box = stage.getBoundingClientRect();
      const css = getComputedStyle(stage);
      const next = { width, height, railHeight: parseFloat(css.getPropertyValue('--rail-h')) || RAIL_HEIGHT, railGap: parseFloat(css.getPropertyValue('--rail-gap')) || RAIL_GAP, displayHeight: display.current?.offsetHeight ?? 0, dx: box.left - Math.floor(box.left), dy: box.top - Math.floor(box.top) };
      setMeasures((was) => Object.keys(next).every((key) => was[key as keyof Measures] === next[key as keyof Measures]) ? was : next);
    };
    measure(stage.clientWidth, stage.clientHeight);
    const observer = new ResizeObserver((entries) => {
      const box = entries[0]?.contentRect;
      if (box !== undefined) measure(box.width, box.height);
    });
    observer.observe(stage);
    // A capped frame can move by half a pixel without its content box changing size.
    const resized = () => { measure(stage.clientWidth, stage.clientHeight); };
    window.addEventListener('resize', resized);
    void document.fonts.ready.then(() => {
      if (!mounted) return;
      measure(stage.clientWidth, stage.clientHeight);
      setFontsReady(true);
    });
    return () => { mounted = false; observer.disconnect(); window.removeEventListener('resize', resized); };
  }, []);

  const present = FILM_ORDER.filter((kind) => films[kind] !== null).join(',');
  const drawing = useMemo<Drawing>(() => {
    const input = { ...measures, present: present.split(',').filter(Boolean) as FilmKind[] };
    return { layout: onPixels(stageLayout({ ...input, moment }), measures), places: placeSpots(onPixels(stageLayout({ ...input, moment: 'films' }), measures)), moment, present, measures };
  }, [measures, moment, present]);

  usePageState('films', reading.length > 0 ? 'landing' : FILM_ORDER.every((kind) => films[kind] !== null) ? 'ready' : 'empty');
  usePageState('ready', fontsReady && measures.width > 0 && measures.height > 0);
  usePageState('drag', held.over);
  usePageState('leaving', leaving);

  useLayoutEffect(() => {
    const part = tiles.current;
    if (part === null) return undefined;
    const was = previous.current;
    const changedMoment = was !== null && (was.moment !== drawing.moment || (moment === 'find' && was.present !== drawing.present));
    const resized = was === null || was.measures !== drawing.measures;
    previous.current = drawing;
    if (changedMoment && !resized && (motionIsReduced(part) || moment === 'three' || was.moment === 'three')) {
      fly(part, new Map(), new Map());
      draw(part, was);
      part.classList.add('fading');
      const timeout = window.setTimeout(() => {
        part.classList.add('still');
        draw(part, drawing);
        readLayout(part);
        part.classList.remove('still', 'fading');
      }, durationOf(part, '--dur-fade'));
      return () => { window.clearTimeout(timeout); part.classList.remove('fading'); };
    }
    const starts = new Map<HTMLElement, ReturnType<typeof flightFrom>>();
    const rests = new Map<HTMLElement, { x: number; y: number }>();
    part.classList.add('still');
    draw(part, drawing);
    if (changedMoment && !resized) for (const kind of FILM_ORDER) {
      const before = was.layout.spots[kind];
      const after = drawing.layout.spots[kind];
      const tile = part.querySelector<HTMLElement>(`.tile[data-place="${kind}"]`);
      if (before === null || after === null || tile === null || after.width === 0) continue;
      starts.set(tile, flightFrom(before, after));
      rests.set(tile, after);
    }
    if (changedMoment && !resized) drawCaptions(part, was);
    fly(part, starts, rests);
    if (changedMoment && !resized) drawCaptions(part, drawing);
    return undefined;
  }, [drawing, moment]);

  useLayoutEffect(() => {
    const part = tiles.current;
    return () => { if (part !== null) fly(part, new Map(), new Map()); };
  }, []);

  const faces = Object.fromEntries(FILM_ORDER.map((kind) => [kind, reading.includes(kind) ? 'reading' : films[kind] === null ? 'empty' : 'filled'])) as Record<FilmKind, 'empty' | 'reading' | 'filled'>;
  return (
    <>
      <h1 ref={display} className="display" style={{ '--head-y': drawing.layout.displayTop } as CSSProperties}>{STAGE_WORDS.display}</h1>
      <FilmPlaces spots={drawing.places} faces={faces} target={held.target} {...(moment === 'films' && onPressPlace !== undefined ? { onPress: onPressPlace } : {})} />
      <div ref={tiles} className={['tiles', marked && 'marked', reviewing && 'reviewing'].filter(Boolean).join(' ')}>
        {FILM_ORDER.map((kind, index) => {
          const film = films[kind];
          const spot = drawing.layout.spots[kind] ?? drawing.places[kind];
          const caption = captionSpot(spot);
          return (
            <Fragment key={kind}>
              {film !== null && <FilmTile kind={kind} index={index} film={moment === 'three' ? { ...film, names: false, on: null, only: null, frame: { kind: 'whole' }, light: null } : film} spot={spot} glide={moment !== 'three'} replay={replay} target={held.target === kind}
                {...(moment === 'find' && !reviewing && onPoint !== undefined ? { onPoint: (key: string | null) => { onPoint(kind, key); } } : {})} />}
              <div className="caption" data-place={kind} aria-hidden="true" data-away={moment === 'find' && film === null || undefined}
                style={{ '--i': index, '--cx': caption.centreX, '--cy': caption.top } as CSSProperties}>{FILM_NAMES[kind]}</div>
            </Fragment>
          );
        })}
      </div>
    </>
  );
}
