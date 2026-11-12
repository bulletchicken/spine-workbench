// The spine with the films it was fitted from, standing in the same space:
// one film as a back wall, two at a right angle, and the view straight
// along a film with no perspective, which is the picture that shows whether
// the bones sit on their outlines.
//
// Every film here is the made-up case's own, placed from its own geometry.
// The outlines drawn over the alignment views are the marks the fit was
// given, drawn by the page from film pixels: nothing about them comes from
// the bones.

import { useEffect, useMemo, useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { viewAlongFilm, viewOf } from '../looking/framing';
import type { CanvasSize } from '../looking/view';
import type { CornersOnFilm } from '../made-up-case/made-up-case';
import { type FilmCamera, type Px, filmToUpright } from '../placing/film-camera';
import { fittedFilmCamera } from '../placing/fitted-film-camera';
import { CASE, Dial, type Parts, Room, useBones, useFilms } from './gallery-parts';
import { Spine3D, type Spine3DFilm } from './Spine3D';

function useBothFilms(which: 'front' | 'both'): { front?: Spine3DFilm; side?: Spine3DFilm } | null {
  const pictures = useFilms();
  return useMemo(() => {
    if (pictures === null) return null;
    const front = { geometry: CASE.front.geometry, size: CASE.front.size, pixels: pictures.front };
    return which === 'front' ? { front } : { front, side: { geometry: CASE.side.geometry, size: CASE.side.size, pixels: pictures.side } };
  }, [pictures, which]);
}

function OneFilm() {
  const bones = useBones();
  const films = useBothFilms('front');
  return <Room size="wide">{bones !== null && films !== null && <Spine3D fit={CASE.fit} bones={bones} films={films} />}</Room>;
}

function TwoFilms() {
  const bones = useBones();
  const films = useBothFilms('both');
  const [parts, setParts] = useState<Parts | null>(null);
  const [hinge, setHinge] = useState(1);
  const [strength, setStrength] = useState(1);

  // Seen from the front and the patient's right: the side this side film is read from, so both films face the viewer.
  useEffect(() => {
    parts?.stage.setView((canvas) => viewOf(parts.stage.bounds(), canvas, { azimuth: -34, elevation: 14, zoom: 0.8, perspective: 1 }));
  }, [parts]);

  useEffect(() => {
    parts?.stage.film('side')?.set({ hinge, strength });
    parts?.stage.film('front')?.set({ strength });
  }, [parts, films, hinge, strength]);

  return (
    <Room
      size="wide"
      under={
        <>
          <Dial name="Side film's hinge" value={hinge} onChange={setHinge} />
          <Dial name="Films' strength" value={strength} onChange={setStrength} />
        </>
      }
    >
      {bones !== null && films !== null && <Spine3D fit={CASE.fit} bones={bones} films={films} onReady={setParts} />}
    </Room>
  );
}

/** How a film is shown in a canvas so that it fills it, with a little room round it. */
function shownIn(camera: FilmCamera, canvas: CanvasSize) {
  const [width, height] = camera.quarterTurns % 2 === 0 ? camera.size : [camera.size[1], camera.size[0]];
  const pxPerFilmPx = Math.min((canvas.width - 24) / width, (canvas.height - 24) / height);
  // The film's first pixel is a whole number at its centre: its outer edge is half a pixel before it.
  return { pxPerFilmPx, firstPixelAt: [(canvas.width - width * pxPerFilmPx) / 2 + 0.5 * pxPerFilmPx, (canvas.height - height * pxPerFilmPx) / 2 + 0.5 * pxPerFilmPx] as const };
}

/** The marks the fit was given, as outlines over the canvas, placed from film pixels alone. */
function Marks<Name extends string>({ camera, vertebrae, order, canvas }: { camera: FilmCamera; vertebrae: CornersOnFilm<Name>[]; order: readonly Name[]; canvas: CanvasSize }) {
  const { pxPerFilmPx, firstPixelAt } = shownIn(camera, canvas);
  const onPage = (mark: Px) => {
    const [u, v] = filmToUpright(camera, mark);
    return `${String(firstPixelAt[0] + u * pxPerFilmPx)},${String(firstPixelAt[1] + v * pxPerFilmPx)}`;
  };
  return (
    <svg className="spine-marks" viewBox={`0 0 ${String(canvas.width)} ${String(canvas.height)}`} aria-hidden="true">
      {vertebrae.map(({ level, marked }) => {
        const points = order.map((name) => onPage(marked[name])).join(' ');
        return (
          <g key={level}>
            <polygon className="spine-marks-casing" points={points} />
            <polygon className="spine-marks-outline" points={points} />
          </g>
        );
      })}
    </svg>
  );
}

const FRONT_CAMERA = fittedFilmCamera(CASE.front.geometry, CASE.front.size, CASE.fit.frontFilm);
if (CASE.fit.sideFilm === null) throw new Error('the pair has no side film placement');
const SIDE_CAMERA = fittedFilmCamera(CASE.side.geometry, CASE.side.size, CASE.fit.sideFilm);
if (FRONT_CAMERA === null || SIDE_CAMERA === null) throw new Error('the pair has no film camera');

function Aligned({ along }: { along: 'front' | 'side' }) {
  const bones = useBones();
  const films = useBothFilms('both');
  const [parts, setParts] = useState<Parts | null>(null);
  const [canvas, setCanvas] = useState<CanvasSize | null>(null);
  const [strength, setStrength] = useState(0.55);
  const [marks, setMarks] = useState(1);
  const camera = along === 'front' ? FRONT_CAMERA : SIDE_CAMERA;
  if (camera === null) throw new Error('the film has no camera');
  // Only the film looked along is hung: the other would stand edge-on across the picture.
  const one = useMemo(() => (films === null ? null : along === 'front' ? { front: films.front } : { side: films.side }), [films, along]);

  useEffect(() => {
    if (parts === null) return undefined;
    const { stage, spine } = parts;
    stage.setView((size) => viewAlongFilm(camera, shownIn(camera, size)));
    spine.setSpine({ shadow: 0 });
    // The marks are laid over the canvas at its size as last drawn.
    return stage.onDrawn(() => {
      const now = stage.size();
      setCanvas((was) => (was !== null && was.width === now.width && was.height === now.height ? was : now));
    });
  }, [parts, camera]);

  useEffect(() => {
    parts?.spine.set({ strength });
  }, [parts, strength]);

  return (
    <Room
      over={
        canvas !== null &&
        marks > 0 && (
          <div style={{ opacity: marks }}>
            {along === 'front' ? (
              <Marks camera={camera} vertebrae={CASE.front.vertebrae} order={['supL', 'supR', 'infR', 'infL']} canvas={canvas} />
            ) : (
              <Marks camera={camera} vertebrae={CASE.side.vertebrae} order={['supA', 'supP', 'infP', 'infA']} canvas={canvas} />
            )}
          </div>
        )
      }
      under={
        <>
          <Dial name="Bones' strength" value={strength} onChange={setStrength} />
          <Dial name="Marks" value={marks} onChange={setMarks} />
        </>
      }
    >
      {bones !== null && one !== null && <Spine3D fit={CASE.fit} bones={bones} films={one} turnable={false} shadow={false} onReady={setParts} />}
    </Room>
  );
}

const page: GalleryPage = {
  area: 'spine-3d',
  part: 'OnTheFilms',
  states: [
    { name: 'One film: the front film as a back wall', show: () => <OneFilm /> },
    { name: 'Two films at a right angle, the side film hinged', show: () => <TwoFilms /> },
    { name: 'Along the front film, no perspective, bones at 55%', show: () => <Aligned along="front" /> },
    { name: 'Along the side film, no perspective, bones at 55%', show: () => <Aligned along="side" /> },
  ],
};

export default page;
