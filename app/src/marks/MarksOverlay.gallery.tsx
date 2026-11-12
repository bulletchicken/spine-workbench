// The outlines in the gallery: the sample case's chain drawn over its own
// films, in each state of Find, each film in the design system's tile at
// its size in Find at 1440 by 900. Every film is a real film viewer, so line
// widths, casings, dashes and names can be judged at any zoom (the wheel, a
// drag and a double-click work).
//
// The system's rules read the moment from an attribute on an ancestor, the
// pulse and the review from classes on .tiles, and the names from the tile's
// data-names. The page is shared by every state, so each state carries its
// own moment on its own .tiles, where the app writes it on <html>.

import { useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import type { FilmKind, FilmMarks, Level, VertebraOnFilm } from '@/shapes';
import { fit, travelTo, type CameraFrame } from '@/film-viewer/camera/camera';
import { makeCameras } from '@/film-viewer/camera/cameras';
import type { CameraIntent } from '@/film-viewer/camera/gestures';
import { FilmViewer, type FilmViewerHandle } from '@/film-viewer/FilmViewer';
import { useLayer } from '@/film-viewer/layer-slot';
import type { GalleryPage } from '@/gallery/gallery-page';
import { needsALook } from '@/review/needs-a-look';
import { SAMPLE_FILMS } from '@/sample-case/films';
import { caseAt } from '@/sample-case/stages';
import { FILM_NAMES } from '@/words/film-names';
import { MARK_WORDS } from '@/words/marks';
import { fromFilmMarks } from './from-film-marks';
import type { Quad } from './mark-to-draw';
import { MarksOverlay } from './MarksOverlay';
import './marks.gallery.css';

/** The sample case's marks on one film, after its search. */
function marksOf(kind: FilmKind): FilmMarks | null {
  return caseAt('pass-result').case.marks.find((marks) => marks.film === kind) ?? null;
}

/** Each vertebra's look and spoken line, by the review rule and the words. */
const read = (vertebra: VertebraOnFilm) => ({ look: needsALook(vertebra) ? ('look' as const) : ('found' as const), spoken: MARK_WORDS.outline(vertebra.level, needsALook(vertebra)) });

const centreOf = (quad: Quad): [number, number] => [(quad[0][0] + quad[1][0] + quad[2][0] + quad[3][0]) / 4, (quad[0][1] + quad[1][1] + quad[2][1] + quad[3][1]) / 4];

/** A Find tile at 1440 by 900 with Front and Side in. */
const TILE = { width: 322, height: 644 };
const NO_MARGINS = { top: 0, right: 0, bottom: 0, left: 0 };

interface TileProps {
  kind: FilmKind;
  /** Levels drawn so far, top down; left out, all of them. */
  upTo?: Level;
  /** The level pointed at, or in focus in a review. */
  on?: Level;
  /** The level reviewed: the rest stand back, and the camera goes to it. */
  only?: Level;
  /** How far the camera is pushed in on the reviewed level. */
  over?: number;
}

export function Tile({ kind, upTo, on, only, over = 2 }: TileProps) {
  const handle = useRef<FilmViewerHandle>(null);
  const sample = SAMPLE_FILMS[kind];
  const pixels = useMemo(() => sample.pixels(), [sample]);
  const film = marksOf(kind);
  const marks = useMemo(() => {
    if (film === null) return [];
    const last = upTo === undefined ? Infinity : film.vertebrae.findIndex((vertebra) => vertebra.level === upTo);
    return fromFilmMarks(film, read, (vertebra) => film.vertebrae.indexOf(vertebra) <= last);
  }, [film, upTo]);
  const keyOf = (level: Level | undefined) => marks.find((mark) => mark.name === level)?.key ?? null;
  const reviewed = marks.find((mark) => mark.name === only);

  const frame: CameraFrame = useMemo(
    () => ({ size: { width: sample.facts.width, height: sample.facts.height }, view: { quarterTurns: 0, mirrored: false }, viewport: TILE, margins: NO_MARGINS }),
    [sample],
  );
  const lookAt = reviewed === undefined ? null : centreOf(reviewed.quad);
  const [cameras] = useState(makeCameras);
  useEffect(
    () =>
      cameras.subscribe((which, moving) => {
        const placement = cameras.placementOf(which);
        if (placement !== null) handle.current?.place(placement, moving);
      }),
    [cameras],
  );
  const onCamera = (intent: CameraIntent) => {
    if (intent.kind === 'zoom') cameras.zoomAt('front', intent.factor, intent.at);
    else if (intent.kind === 'pan') cameras.panBy('front', intent.by);
    else cameras.fit('front', false);
  };

  const outlines = useLayer('outlines', MarksOverlay, {
    label: MARK_WORDS.outlinesOn(kind),
    marks,
    on: keyOf(on),
    only: keyOf(only),
  });

  // The tile's own geometry, as the stage writes it: its real size at rest.
  const place = { '--bw': TILE.width, '--bh': TILE.height, '--x': 0, '--y': 0, '--s': 1, '--i': 0 } as CSSProperties;
  return (
    <figure className="marks-stage">
      <div className="marks-stage-room" style={{ width: TILE.width, height: TILE.height }}>
        <div className="tile in" data-names={kind === 'front' || kind === 'side' ? 'on' : 'off'} style={place}>
          <div className="film">
            <FilmViewer
              ref={handle}
              pixels={pixels}
              placement={lookAt === null ? fit(frame) : travelTo(frame, lookAt, over)}
              layers={[outlines]}
              onCamera={onCamera}
              onViewport={(viewport) => {
                cameras.hang('front', { ...frame, viewport });
                if (lookAt !== null) cameras.travelTo('front', lookAt, over, false);
              }}
              label={`${FILM_NAMES[kind]} film`}
            />
          </div>
        </div>
      </div>
      <figcaption className="marks-stage-caption">{FILM_NAMES[kind]}</figcaption>
    </figure>
  );
}

interface MomentProps {
  /** The moment the films are in: outside Find the outlines fade and are hidden. */
  moment?: 'find' | 'films';
  /** The last outline has appeared: the ones that need a look pulse once. */
  marked?: boolean;
  /** A review is on: every outline but the one in focus steps back. */
  reviewing?: boolean;
  children: ReactNode;
}

/** The films' row, as the stage holds it: the moment and the classes the system's rules read. */
function Tiles({ moment = 'find', marked = false, reviewing = false, children }: MomentProps) {
  return (
    <div data-moment={moment} className={['tiles marks-stage-row', marked && 'marked', reviewing && 'reviewing'].filter((each) => each !== false).join(' ')}>
      {children}
    </div>
  );
}

/** Front and Side, side by side, in the same state. */
export function Pair({ moment = 'find', marked = false, reviewing = false, ...props }: Omit<TileProps, 'kind'> & Omit<MomentProps, 'children'>) {
  return (
    <Tiles moment={moment} marked={marked} reviewing={reviewing}>
      <Tile {...props} kind="front" />
      <Tile {...props} kind="side" />
    </Tiles>
  );
}

const page: GalleryPage = {
  area: 'marks',
  part: 'MarksOverlay',
  states: [
    { name: 'Found: every outline over its casing, the ones that need a look in amber, pulsing once', show: () => <Pair marked /> },
    { name: 'The replay part-way: outlines down to T9 drawn', show: () => <Pair upTo="T9" /> },
    { name: 'Pointing at T7: the same vertebra lights on both films', show: () => <Pair marked on="T7" /> },
    { name: 'A review: T3 in focus, the rest standing back', show: () => <Pair marked reviewing only="T3" /> },
    {
      name: 'A bending film: outlines, no names',
      show: () => (
        <Tiles marked>
          <Tile kind="bend-left" />
        </Tiles>
      ),
    },
    { name: 'Away from Find: faded and hidden', show: () => <Pair moment="films" marked /> },
  ],
};

export default page;
