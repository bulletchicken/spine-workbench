// The film viewer in the gallery: the viewer's own test film with a test
// layer over it, so that a mark's lock to the film can be judged by eye
// at every zoom, pan and turn. The layer rings each lone bright pixel
// with a circle centred on its pixel centre, outlines the vertebrae and
// the film's outer edge, and keeps its line widths and labels the same
// size on screen at fit and at 600%. The rings and labels size themselves
// from the viewer's --film-zoom, so they hold their size through a glide
// too. Wheel, drag, double-click and right-drag all work here, each
// viewer with cameras of its own.

import { useEffect, useMemo, useRef, useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { fit, travelTo, type CameraFrame, type Margins } from './camera/camera';
import { makeCameras } from './camera/cameras';
import type { CameraIntent } from './camera/gestures';
import type { FilmLayerProps, FilmView } from './film-space';
import { FilmViewer, type FilmViewerHandle } from './FilmViewer';
import { PLAIN_LOOK, type FilmLook } from './gpu/film-shader';
import { useLayer } from './layer-slot';
import { storedUpsideDown, testFilm, type TestFilm } from './test-film';
import './FilmViewer.gallery.css';

/** A main bay's viewport at 1440 by 900, and the margins the bench keeps there for the edge ruler. */
const BAY = { width: 340, height: 740 };
const BAY_MARGINS: Margins = { top: 12, right: 8, bottom: 12, left: 34 };

/**
 * A transform that moves to a film point and undoes the film's turn and
 * mirror there, so what is drawn inside reads upright on screen.
 */
function uprightAt(space: FilmLayerProps['space'], [x, y]: [number, number]): string {
  const [a, b, c, d] = space.matrix.map((value) => value / space.placement.zoom);
  // A turn or a mirror is orthonormal, so its inverse is its transpose.
  const [ia, ib, ic, id] = [a ?? 1, c ?? 0, b ?? 0, d ?? 1];
  return `matrix(${String(ia)} ${String(ib)} ${String(ic)} ${String(id)} ${String(x)} ${String(y)})`;
}

function TestMarks({ film, space }: { film: TestFilm } & FilmLayerProps) {
  const { width, height } = film.size;
  /**
   * The film point level with the outline's middle at its leftmost edge
   * on screen. Zoom and pan scale and move the screen evenly, so this
   * point depends only on the turn and mirror, and holds through a glide.
   */
  const labelAt = (corners: [number, number][], centre: [number, number]): [number, number] => {
    const left = Math.min(...corners.map((corner) => space.toScreen(corner)[0]));
    return space.toFilm([left, space.toScreen(centre)[1]]);
  };
  return (
    <g className="test-marks">
      <rect className="test-marks__edge" x={-0.5} y={-0.5} width={width} height={height} />
      {film.vertebrae.map(({ name, corners, centre }) => (
        <g key={name}>
          <polygon className="test-marks__casing" points={corners.map((p) => p.join(',')).join(' ')} />
          <polygon className="test-marks__outline" points={corners.map((p) => p.join(',')).join(' ')} />
          <g transform={uprightAt(space, labelAt(corners, centre))}>
            <g className="test-marks__screen-sized">
              <text className="test-marks__label" x={-8}>
                {name}
              </text>
            </g>
          </g>
        </g>
      ))}
      {film.probes.map(([x, y]) => (
        <g key={`${String(x)},${String(y)}`} transform={`translate(${String(x)} ${String(y)})`}>
          <g className="test-marks__screen-sized">
            <circle className="test-marks__probe" r={6} />
            <line className="test-marks__cross" x1={-12} x2={12} />
            <line className="test-marks__cross" y1={-12} y2={12} />
          </g>
        </g>
      ))}
    </g>
  );
}

interface StageProps {
  film?: TestFilm;
  view?: FilmView;
  /** Zoom as a multiple of the fit, and the film point to centre there. */
  over?: number;
  at?: [number, number];
  look?: FilmLook;
  box?: { width: number; height: number };
  margins?: Margins;
  label?: string;
}

/** Held once: a default made afresh on each render would make the cameras afresh too, and lose their zoom. */
const AS_STORED: FilmView = { quarterTurns: 0, mirrored: false };

function Stage({
  film = testFilm(),
  view = AS_STORED,
  over = 1,
  at,
  look = PLAIN_LOOK,
  box = BAY,
  margins = BAY_MARGINS,
  label = 'Test film',
}: StageProps) {
  const handle = useRef<FilmViewerHandle>(null);
  const frame: CameraFrame = useMemo(() => ({ size: film.size, view, viewport: box, margins }), [film, view, box, margins]);
  const start = useMemo(() => (at === undefined ? fit(frame) : travelTo(frame, at, over)), [frame, at, over]);
  const cameras = useMemo(() => {
    const made = makeCameras();
    made.hang('front', frame);
    if (at !== undefined) made.travelTo('front', at, over, false);
    return made;
  }, [frame, at, over]);
  const [zoomWord, setZoomWord] = useState(over === 1 ? 'Fit' : `${String(Math.round(over * 100))}%`);

  useEffect(
    () =>
      cameras.subscribe((film, moving) => {
        const placement = cameras.placementOf(film);
        if (placement === null) return;
        handle.current?.place(placement, moving);
        const ratio = cameras.zoomOverFitOf(film) ?? 1;
        setZoomWord(Math.abs(ratio - 1) < 1e-6 ? 'Fit' : `${String(Math.round(ratio * 100))}%`);
      }),
    [cameras],
  );

  const onCamera = (intent: CameraIntent) => {
    if (intent.kind === 'zoom') cameras.zoomAt('front', intent.factor, intent.at);
    else if (intent.kind === 'pan') cameras.panBy('front', intent.by);
    else cameras.fit('front', false);
  };

  const marks = useLayer('test-marks', TestMarks, { film });

  return (
    <figure className="film-viewer-stage">
      <div className="film-viewer-stage__bay" style={{ width: box.width, height: box.height }}>
        <FilmViewer
          ref={handle}
          pixels={film.pixels}
          placement={start}
          layers={[marks]}
          look={look}
          onCamera={onCamera}
          onViewport={(viewport) => {
            cameras.hang('front', { ...frame, viewport });
          }}
          label={label}
        />
      </div>
      <figcaption className="film-viewer-stage__caption">
        <span>{label}</span>
        <span className="film-viewer-stage__zoom">{zoomWord}</span>
      </figcaption>
    </figure>
  );
}

/** Four films hung as in step 1 at 1440 by 900: bays 236 by 576, the film 8 pixels in from each side. */
function FourFilms({ box = { width: 236, height: 576 } }: { box?: { width: number; height: number } }) {
  const film = testFilm();
  const margins = { top: 8, right: 8, bottom: 8, left: 8 };
  return (
    <div className="film-viewer-row">
      {['Front', 'Side', 'Bending left', 'Bending right'].map((name, i) => (
        <Stage
          key={name}
          film={i === 2 ? storedUpsideDown(film) : film}
          box={box}
          margins={margins}
          label={name}
          view={{ quarterTurns: i === 2 ? 2 : 0, mirrored: false }}
        />
      ))}
    </div>
  );
}

/** A film small enough that 600% of its fit is past 1:1, so single pixels show as squares. */
const SMALL = { width: 300, height: 700 };

const page: GalleryPage = {
  area: 'film-viewer',
  part: 'FilmViewer',
  states: [
    { name: 'At fit', show: () => <Stage /> },
    { name: 'At 600%, on the probe pixels', show: () => <Stage over={6} at={[750, 400]} label="Test film at 600%" /> },
    { name: 'At 600%, at the top-left corner', show: () => <Stage over={6} at={[0, 0]} label="The film's first pixel" /> },
    {
      name: 'Stored upside down, shown upright',
      show: () => <Stage film={storedUpsideDown(testFilm())} view={{ quarterTurns: 2, mirrored: false }} label="Stored upside down" />,
    },
    { name: 'A quarter turn, at 600%', show: () => <Stage view={{ quarterTurns: 1, mirrored: false }} over={6} at={[750, 400]} box={{ width: 740, height: 340 }} label="Turned a quarter" /> },
    { name: 'Window narrowed, brightness flipped', show: () => <Stage look={{ centre: 0.55, width: 0.6, flipped: true }} label="Flipped" /> },
    { name: 'Past 1:1: single pixels, ringed at their centres', show: () => <Stage film={testFilm(SMALL)} over={6} at={[76, 41]} label="A small film at 600%" /> },
    { name: 'Four films on screen', show: () => <FourFilms /> },
    { name: 'Four films on screen, at 1024 wide', show: () => <FourFilms box={{ width: 150, height: 357 }} /> },
  ],
};

export default page;
