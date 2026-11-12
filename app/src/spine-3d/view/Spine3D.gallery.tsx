// The spine in 3D in the gallery: a made-up spine, fitted by the engine,
// in each of its named views, with a set of vertebrae highlighted, and
// under the pointer. Every one can be turned and zoomed by hand.

import { useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import type { Level } from '@/shapes';
import type { ViewName } from '../looking/named-views';
import { CASE, Room, useBones } from './gallery-parts';
import { Spine3D } from './Spine3D';

const degrees = (radians: number) => `${((radians * 180) / Math.PI).toFixed(1)}°`;

/** What the fit says of the vertebra pointed at: its angles, each with its band, and what was not measured. */
function AboutLevel({ level }: { level: Level | null }) {
  const vertebra = CASE.fit.vertebrae.find((each) => each.level === level);
  if (vertebra === undefined) return <p className="gallery-note">Point at a vertebra. Drag to turn, wheel or pinch to zoom.</p>;
  const { angles, sd, inferred } = vertebra;
  const taken = [inferred.depth && 'depth', inferred.width && 'width', inferred.twist && 'twist'].filter((word) => word !== false);
  return (
    <p className="gallery-note">
      {vertebra.level}: side tilt {degrees(angles.sideTiltRad)} ± {degrees(sd.anglesRad[0])}, forward lean {degrees(angles.forwardLeanRad)} ± {degrees(sd.anglesRad[1])}, twist{' '}
      {degrees(angles.twistRad)} ± {degrees(sd.anglesRad[2])}. {taken.length === 0 ? 'Nothing was inferred.' : `Inferred: ${taken.join(', ')}.`}
    </p>
  );
}

function Opening() {
  const bones = useBones();
  const [pointed, setPointed] = useState<Level | null>(null);
  return (
    <Room under={<AboutLevel level={pointed} />}>
      {bones !== null && <Spine3D fit={CASE.fit} bones={bones} onPoint={setPointed} />}
    </Room>
  );
}

function Named({ view }: { view: ViewName }) {
  const bones = useBones();
  return <Room size="small">{bones !== null && <Spine3D fit={CASE.fit} bones={bones} view={view} label={`The spine in 3D, ${view} view`} />}</Room>;
}

function ThreeViews() {
  return (
    <div className="spine-row">
      <Named view="front" />
      <Named view="side" />
      <Named view="top" />
    </div>
  );
}

/** The vertebrae a thoracic curve is measured across, as a measurement would light them. */
const MEASURED_ACROSS: readonly Level[] = ['T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12'];

function Highlighted() {
  const bones = useBones();
  return <Room>{bones !== null && <Spine3D fit={CASE.fit} bones={bones} highlighted={MEASURED_ACROSS} view="front" />}</Room>;
}

const page: GalleryPage = {
  area: 'spine-3d',
  part: 'Spine3D',
  states: [
    { name: 'The view it opens in', show: () => <Opening /> },
    { name: 'Front, side and top', show: () => <ThreeViews /> },
    { name: 'A highlighted set', show: () => <Highlighted /> },
  ],
};

export default page;
