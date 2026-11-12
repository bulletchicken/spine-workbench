// How a spine can be made to arrive, on sliders: its see-through line
// version, each bone's fill from the end the spine fills from, the whole spine's fill
// by height, and a bone pressed flat on its film and stood out to where the
// fit puts it. The view moves nothing by itself: each slider is a value set
// from outside, as a transition's one progress number would set them.

import { useEffect, useMemo, useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { inTurn, settle, stretch } from '../motion/easing';
import { CASE, Dial, type Parts, Room, useBones, useFilms } from './gallery-parts';
import { Spine3D, type Spine3DFilm } from './Spine3D';

function SeeThroughAndFill() {
  const bones = useBones();
  const [parts, setParts] = useState<Parts | null>(null);
  const [cage, setCage] = useState(1);
  const [fill, setFill] = useState(0.55);
  const [spineFill, setSpineFill] = useState(1);
  const [strength, setStrength] = useState(1);

  useEffect(() => {
    parts?.spine.set({ cage, fill, strength });
    parts?.spine.setSpine({ fill: spineFill });
  }, [parts, cage, fill, spineFill, strength]);

  return (
    <Room
      under={
        <>
          <Dial name="See-through lines" value={cage} onChange={setCage} />
          <Dial name="Fill, each bone" value={fill} onChange={setFill} />
          <Dial name="Fill, whole spine" value={spineFill} onChange={setSpineFill} />
          <Dial name="Strength" value={strength} onChange={setStrength} />
        </>
      }
    >
      {bones !== null && <Spine3D fit={CASE.fit} bones={bones} shadow={false} onReady={setParts} />}
    </Room>
  );
}

/** One number runs the whole arrival: each bone's lines draw in, top to bottom, then it fills, and the shadow comes last. */
function FromOneNumber() {
  const bones = useBones();
  const [parts, setParts] = useState<Parts | null>(null);
  const [progress, setProgress] = useState(0.5);

  useEffect(() => {
    if (parts === null) return;
    const { spine } = parts;
    const lines = stretch(progress, 0, 0.55);
    const filling = stretch(progress, 0.3, 0.9);
    spine.levels.forEach((level, i) => {
      const drawn = settle(inTurn(lines, i, spine.levels.length, 5));
      const filled = settle(inTurn(filling, i, spine.levels.length, 5));
      spine.set({ cage: drawn * (1 - filled), fill: filled, reveal: 1 }, level);
    });
    spine.setSpine({ shadow: settle(stretch(progress, 0.9, 1)) });
  }, [parts, progress]);

  return <Room under={<Dial name="Progress" value={progress} onChange={setProgress} />}>{bones !== null && <Spine3D fit={CASE.fit} bones={bones} onReady={setParts} />}</Room>;
}

function FlatToFitted() {
  const bones = useBones();
  const pictures = useFilms();
  const [parts, setParts] = useState<Parts | null>(null);
  const [pose, setPose] = useState(0.5);
  const [depth, setDepth] = useState(1);
  const [inOrder, setInOrder] = useState(1);
  const films = useMemo<{ front?: Spine3DFilm }>(() => (pictures === null ? {} : { front: { geometry: CASE.front.geometry, size: CASE.front.size, pixels: pictures.front } }), [pictures]);

  useEffect(() => {
    if (parts === null) return;
    const { spine } = parts;
    spine.levels.forEach((level, i) => {
      spine.set({ pose: pose * settle(inTurn(inOrder, i, spine.levels.length, 4)) }, level);
    });
    spine.setSpine({ depth });
  }, [parts, pose, depth, inOrder]);

  return (
    <Room
      size="wide"
      under={
        <>
          <Dial name="Flat to fitted" value={pose} onChange={setPose} />
          <Dial name="Depth" value={depth} onChange={setDepth} />
          <Dial name="One by one" value={inOrder} onChange={setInOrder} />
        </>
      }
    >
      {bones !== null && pictures !== null && <Spine3D fit={CASE.fit} bones={bones} films={films} shadow={false} onReady={setParts} />}
    </Room>
  );
}

const page: GalleryPage = {
  area: 'spine-3d',
  part: 'Forming',
  states: [
    { name: 'See-through and fill, on sliders', show: () => <SeeThroughAndFill /> },
    { name: 'A whole arrival from one number', show: () => <FromOneNumber /> },
    { name: 'Flat on the film to fitted, and depth, on sliders', show: () => <FlatToFitted /> },
  ],
};

export default page;
