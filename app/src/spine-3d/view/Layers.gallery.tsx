import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { CylinderGeometry, Mesh } from 'three';
import type { GalleryPage } from '@/gallery/gallery-page';
import type { Bone } from '../bones/read-bones';
import type { DrawnSpine } from '../scene/drawn-spine';
import { attachHandTurning } from '../scene/hand-turning';
import { ACCENT, CAGE, GROUND } from '../scene/look';
import { modelSpine } from '../scene/spine-model';
import { makeStage, type Stage } from '../scene/stage';
import { CASE, Dial, Room, useBones } from './gallery-parts';
import './Spine3D.css';

interface Drawing { stage: Stage; main: DrawnSpine; other: DrawnSpine; ghost: DrawnSpine }

function LayerDrawing({ bones, strength, columns, ghost, progress }: { bones: readonly Bone[]; strength: number; columns: boolean; ghost: boolean; progress: number }) {
  const canvas = useRef<HTMLCanvasElement>(null), drawing = useRef<Drawing | null>(null);
  useLayoutEffect(() => {
    if (canvas.current === null) return;
    const stage = makeStage(canvas.current), model = modelSpine(CASE.fit, bones);
    const main = stage.addSpine(model), other = stage.addSpine(model, { shapesFrom: main }), faint = stage.addSpine(model, { shapesFrom: main, flat: true });
    const own: CylinderGeometry[] = [];
    for (const spine of [main, other]) {
      const layer = spine.addLayer(), metal = layer.lit(GROUND), ink = layer.flat(ACCENT, .8);
      for (const level of spine.levels.filter((_, i) => i % 3 === 0)) {
        const geometry = new CylinderGeometry(2.5, 2.5, 32, 12); own.push(geometry);
        const screw = new Mesh(geometry, metal); screw.rotation.z = Math.PI / 2;
        layer.onBone(level).add(screw); layer.pointable(screw, `anchor:${level}`);
      }
      const low = model.heights[0], high = model.heights[1];
      for (const [i, side] of [-1, 1].entries()) {
        const geometry = new CylinderGeometry(2, 2, high - low, 12); own.push(geometry);
        const rod = new Mesh(geometry, i === 0 ? metal : ink); rod.rotation.x = Math.PI / 2;
        rod.position.set(-20, side * 15, (low + high) / 2); layer.onSpine.add(rod);
        layer.pointable(rod, `rod:${String(i)}`); if (i === 1) layer.inFront(rod, true);
      }
      layer.changed();
    }
    faint.setSpine({ flatColour: CAGE.onPage, strength: .07, shadow: 0 });
    faint.betweenPoses(CASE.fit.vertebrae.map((v) => v.pose), CASE.fit.vertebrae.map((v) => ({ ...v.pose, centre: [v.pose.centre[0], v.pose.centre[1] + 20, v.pose.centre[2]] })), 1);
    stage.setView(() => ({ ...stage.namedView('front'), azimuth: 180 }));
    const hand = attachHandTurning(stage, canvas.current);
    drawing.current = { stage, main, other, ghost: faint };
    return () => { drawing.current = null; hand.detach(); stage.dispose(); model.dispose(); own.forEach((shape) => { shape.dispose(); }); };
  }, [bones]);
  useEffect(() => {
    const parts = drawing.current; if (parts === null) return;
    parts.main.set({ strength }); parts.other.set({ strength });
    parts.other.setSpine({ visible: columns }); parts.ghost.setSpine({ visible: ghost });
    const from = CASE.fit.vertebrae.map((v) => v.pose), to = from.map((pose) => ({ ...pose, centre: [pose.centre[0], pose.centre[1] * .4, pose.centre[2]] as [number, number, number] }));
    parts.other.betweenPoses(from, to, progress);
    parts.stage.inColumns(columns ? [parts.main, parts.other] : [], { gapPx: 0, footPx: 0 });
  }, [strength, columns, ghost, progress]);
  return <div className="spine-3d"><canvas ref={canvas} className="spine-3d-canvas" /></div>;
}

function Layers() {
  const bones = useBones(), [strength, setStrength] = useState(.55), [columns, setColumns] = useState(false), [ghost, setGhost] = useState(true), [progress, setProgress] = useState(.5);
  return <Room size="wide" under={<>
    <Dial name="Bone strength" value={strength} onChange={setStrength} />
    <Dial name="Pose progress" value={progress} onChange={setProgress} />
    <label><input type="checkbox" checked={columns} onChange={(e) => { setColumns(e.target.checked); }} />Two columns</label>
    <label><input type="checkbox" checked={ghost} onChange={(e) => { setGhost(e.target.checked); }} />Ghost</label>
  </>}>{bones !== null && <LayerDrawing bones={bones} strength={strength} columns={columns} ghost={ghost} progress={progress} />}</Room>;
}

const page: GalleryPage = { area: 'spine-3d', part: 'Layers', states: [{ name: 'Objects through a bone', show: () => <Layers /> }] };
export default page;
