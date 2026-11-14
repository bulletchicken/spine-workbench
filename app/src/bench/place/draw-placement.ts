import { BufferGeometry, CurvePath, CylinderGeometry, Group, Line, LineCurve3, Mesh, Quaternion, SphereGeometry, TorusGeometry, TubeGeometry, Vector3 } from 'three';
import type { ImplantLayout, SpineFit } from '@/shapes';
import type { DrawnSpine } from '@/spine-3d/scene/drawn-spine';
import type { SceneLayer } from '@/spine-3d/scene/scene-layer';
import { FORMATION } from '@/spine-3d/scene/look';

/** Symbolic heads and hooks; only screw shafts and rods use supplied dimensions. */
export const PLACE_DRAWING = { bone: .55, reviewBone: .34, ink: .8, headRadius: 3.2, headDepth: 2, hookRadius: 4, hookWidth: .7, emptyWidth: .25, screwSegments: 10, ringRadiusPx: 10, ringWidthPx: 1.5, staggerMs: 60, arrivalMs: 1200, backAzimuth: 146, backElevation: 14 } as const;
export interface DrawnPlacement { layer: SceneLayer; parts: Group[]; reveal(count: number, strength: number): void; remove(): void }
export function drawPlacement(spine: Pick<DrawnSpine, 'levels' | 'addLayer'>, fit: SpineFit, layout: ImplantLayout, unsettled: readonly string[]): DrawnPlacement {
  const layer = spine.addLayer(), geometries: BufferGeometry[] = [], parts: Group[] = [];
  const own = <T extends BufferGeometry>(geometry: T): T => { geometries.push(geometry); return geometry; };
  const paint = layer.flat(FORMATION.ink, PLACE_DRAWING.ink);
  const add = (group: Group, id: string, parent: Group) => { parent.add(group); layer.pointable(group, id); if (id === 'upper') parts.unshift(group); else if (id === 'lower') parts.splice(1, 0, group); else parts.push(group); };
  for (const [index, anchor] of layout.anchors.entries()) {
    const vertebra = fit.vertebrae.find((v) => v.level === anchor.level);
    if (vertebra === undefined || !spine.levels.includes(anchor.level)) continue;
    const group = new Group(), rotation = new Vector3(...vertebra.pose.rotation), angle = rotation.length();
    const inverse = angle === 0 ? new Quaternion() : new Quaternion().setFromAxisAngle(rotation.normalize(), angle).invert();
    group.position.copy(new Vector3(...anchor.entryMm).sub(new Vector3(...vertebra.pose.centre)).applyQuaternion(inverse));
    const direction = new Vector3(...anchor.directionUnit).applyQuaternion(inverse).normalize();
    group.quaternion.setFromUnitVectors(new Vector3(0, 1, 0), direction);
    if (anchor.kind === 'screw') {
      if (anchor.diameterMm !== null && anchor.lengthMm !== null) {
        const shaft = new Mesh(own(new CylinderGeometry(anchor.diameterMm / 2, anchor.diameterMm / 2, anchor.lengthMm, PLACE_DRAWING.screwSegments)), paint);
        shaft.position.y = anchor.lengthMm / 2; group.add(shaft);
      }
      const head = new Mesh(own(new CylinderGeometry(PLACE_DRAWING.headRadius, PLACE_DRAWING.headRadius, PLACE_DRAWING.headDepth, PLACE_DRAWING.screwSegments)), paint); group.add(head);
    } else {
      const ring = new Mesh(own(new TorusGeometry(PLACE_DRAWING.hookRadius, anchor.kind === 'hook' ? PLACE_DRAWING.hookWidth : PLACE_DRAWING.emptyWidth, 6, 24, anchor.kind === 'hook' ? Math.PI * 1.5 : Math.PI * 2)), paint);
      ring.rotation.x = Math.PI / 2; group.add(ring);
    }
    add(group, `anchor:${String(index)}`, layer.onBone(anchor.level));
  }
  for (const [upper, level] of [[true, layout.upper], [false, layout.lower]] as const) {
    const v = fit.vertebrae.find((v) => v.level === level); if (v === undefined || !spine.levels.includes(level)) continue;
    const group = new Group(), width = upper ? v.pose.topHalfWidth : v.pose.bottomHalfWidth, z = v.pose.height / 2 * (upper ? 1 : -1);
    const pieces = unsettled.includes(level) ? 8 : 1;
    for (let i = 0; i < pieces; i++) {
      const start = -width + 2 * width * i / pieces, end = pieces === 1 ? width : start + width / pieces;
      group.add(new Line(own(new BufferGeometry().setFromPoints([new Vector3(0, start, z), new Vector3(0, end, z)])), paint));
    }
    add(group, upper ? 'upper' : 'lower', layer.onBone(level));
  }
  // A missing standing line stays missing. The contoured line belongs to a later moment.
  for (const rod of layout.rods) {
    if (rod.standingPointsMm.length < 2) continue;
    const path = new CurvePath<Vector3>();
    for (let i = 1; i < rod.standingPointsMm.length; i++) {
      const a = rod.standingPointsMm[i - 1], b = rod.standingPointsMm[i];
      if (a !== undefined && b !== undefined) path.add(new LineCurve3(new Vector3(...a), new Vector3(...b)));
    }
    const group = new Group(); group.add(new Mesh(own(new TubeGeometry(path, rod.standingPointsMm.length, rod.diameterMm / 2, PLACE_DRAWING.screwSegments, false)), paint));
    const end = rod.standingPointsMm.at(-1), start = rod.standingPointsMm[0];
    for (const at of [start, end]) if (at !== undefined) { const cap = new Mesh(own(new SphereGeometry(rod.diameterMm / 2, 10, 6)), paint); cap.position.set(...at); group.add(cap); }
    add(group, `rod:${rod.side}`, layer.onSpine);
  }
  layer.changed();
  return { layer, parts, reveal(count, strength) { parts.forEach((part, i) => { part.visible = i < count; }); paint.opacity = PLACE_DRAWING.ink * strength; layer.changed(); }, remove() { layer.remove(); for (const geometry of geometries) geometry.dispose(); } };
}
