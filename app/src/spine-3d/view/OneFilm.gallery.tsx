// A front-only fit beside the facts it reports. No mark on the model
// suggests that a typical depth, lean or twist was measured on the film.

import { useEffect, useMemo, useState } from 'react';
import type { Level } from '@/shapes';
import type { GalleryPage } from '@/gallery/gallery-page';
import { GENERIC_BONES_NOTE, ONE_FILM_WORDS as words } from '@/words/spine-3d';
import { viewAlongFilm } from '../looking/framing';
import { FRONT_ONLY_CASE as CASE } from '../made-up-case/made-up-case';
import { cornerTiltSpread } from '../placing/corner-tilt-spread';
import { fittedFilmCamera } from '../placing/fitted-film-camera';
import { type Parts, Room, useBones, useFilms } from './gallery-parts';
import { Spine3D } from './Spine3D';

function cameraOfTheCase() {
  const camera = fittedFilmCamera(CASE.front.geometry, CASE.front.size, CASE.fit.frontFilm);
  if (camera === null) throw new Error('the front film has no camera');
  return camera;
}

const CAMERA = cameraOfTheCase();
const SPREAD = CASE.front.markSpread;
if (SPREAD.kind !== 'millimetres') throw new Error('the made-up corner spread is in millimetres');
const CORNER_SD = new Map(CASE.front.vertebrae.map(({ level, marked }) => [level, cornerTiltSpread(CAMERA, marked, SPREAD.sdMm)]));
const degrees = (radians: number) => `${((radians * 180) / Math.PI).toFixed(1)}°`;
const band = (radians: number, sd: number) => `${degrees(radians)} ± ${degrees(sd)}`;
const cornerBand = (level: Level, angle: number) => {
  const sd = CORNER_SD.get(level);
  return sd === undefined ? words.absent : band(angle, sd);
};
const flag = (value: boolean) => (value ? words.yes : words.no);

/** Plain values from the fit, beside the view, with no marks styled on any bone. */
function Readings() {
  return (
    <div>
      <p>{GENERIC_BONES_NOTE}</p>
      <p>{words.measured}</p>
      <p>{words.typical}</p>
      <p>{words.twist}</p>
      <p>{CASE.fit.lengths === 'measured' ? words.measuredSize : words.typicalSize}</p>
      <p>{words.lengths}: {CASE.fit.lengths}; {words.curve}: {CASE.fit.typicalCurveDeg === null ? words.absent : `${CASE.fit.typicalCurveDeg.toFixed(1)}°`}</p>
      <p>{words.cornerBand}</p>
      <table>
        <thead>
          <tr>
            {[words.level, words.cornerTilt, words.modelTilt, words.lean, words.twistAngle, words.depthFlag, words.widthFlag, words.twistFlag].map((name) => <th key={name}>{name}</th>)}
          </tr>
        </thead>
        <tbody>
          {CASE.fit.vertebrae.map(({ level, cornerTiltRad, angles, sd, inferred }) => (
            <tr key={level}>
              <th scope="row">{level}</th>
              <td>{cornerTiltRad === null ? words.absent : cornerBand(level, cornerTiltRad)}</td>
              <td>{band(angles.sideTiltRad, sd.anglesRad[0])}</td>
              <td>{band(angles.forwardLeanRad, sd.anglesRad[1])}</td>
              <td>{band(angles.twistRad, sd.anglesRad[2])}</td>
              <td>{flag(inferred.depth)}</td>
              <td>{flag(inferred.width)}</td>
              <td>{flag(inferred.twist)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function OneFilm({ aligned }: { aligned: boolean }) {
  const bones = useBones();
  const pictures = useFilms();
  const [parts, setParts] = useState<Parts | null>(null);
  const films = useMemo(() => pictures === null ? {} : { front: { geometry: CASE.front.geometry, size: CASE.front.size, pixels: pictures.front } }, [pictures]);

  useEffect(() => {
    if (!aligned || parts === null) return;
    parts.stage.setView((canvas) => {
      const [width, height] = CAMERA.quarterTurns % 2 === 0 ? CAMERA.size : [CAMERA.size[1], CAMERA.size[0]];
      const pxPerFilmPx = Math.min((canvas.width - 24) / width, (canvas.height - 24) / height);
      return viewAlongFilm(CAMERA, {
        pxPerFilmPx,
        firstPixelAt: [(canvas.width - width * pxPerFilmPx) / 2 + 0.5 * pxPerFilmPx, (canvas.height - height * pxPerFilmPx) / 2 + 0.5 * pxPerFilmPx],
      });
    });
    parts.spine.set({ strength: 0.55 });
  }, [aligned, parts]);

  return (
    <div className="spine-row">
      <Room>{bones !== null && pictures !== null && <Spine3D fit={CASE.fit} bones={bones} films={films} shadow={!aligned} turnable={!aligned} onReady={setParts} />}</Room>
      <Readings />
    </div>
  );
}

const page: GalleryPage = {
  area: 'spine-3d',
  part: 'OneFilm',
  states: [
    { name: words.aligned, show: () => <OneFilm aligned /> },
    { name: words.opening, show: () => <OneFilm aligned={false} /> },
  ],
};

export default page;
