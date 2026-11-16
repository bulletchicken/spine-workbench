import { BufferAttribute, BufferGeometry, CatmullRomCurve3, CylinderGeometry, Group, Line, Matrix4, Mesh, Quaternion, SphereGeometry, TorusGeometry, Vector3 } from 'three';
import type { AnchorProposal, ImplantLayout, SpineFit, PartResult, RodLine, RodProposal, VertebraPose } from '@/shapes';
import type { DrawnSpine } from '@/spine-3d/scene/drawn-spine';
import type { SceneLayer } from '@/spine-3d/scene/scene-layer';
import { anchorLoad, anchorPoint, poseMotion, seatPoint } from './load-view';
import { FORMATION } from '@/spine-3d/scene/look';

/**
 * Symbolic heads and hooks; only screw shafts, rods and the head offset use supplied dimensions. A head is a cup twice the rod's
 * diameter across and along its screw, centred on the rod's seat. Without a rod on its side a head is a symbolic disc at the entry.
 * A rod is drawn as a centripetal Catmull-Rom through its supplied points, `knotStepsMm` apart at least (seats and ends always kept),
 * `rodSteps` rings per knot: drawing tolerances of this project, not measurements.
 */
export const PLACE_DRAWING = { bone: .55, reviewBone: .34, ink: .8, headRadius: 3.2, headDepth: 2, headOverRod: 2, hookRadius: 4, hookWidth: .7, emptyWidth: .25, screwSegments: 16, rodSegments: 16, knotStepsMm: 1.5, rodSteps: 2, ringRadiusPx: 10, ringWidthPx: 1.5, staggerMs: 60, arrivalMs: 1200, backAzimuth: 146, backElevation: 14, endHandleRadiusPx: 12, metal: { colour: '#5d5d5d', roughness: .35, metalness: .5 } } as const;
export interface DrawnPlacement { layer: SceneLayer; parts: Group[]; reveal(count: number, strength: number): void; fadeParts(ids:readonly string[],progress:number):void; compareInk(changed:readonly string[]):void; poses(poses: readonly VertebraPose[], loaded: boolean, rods: RodLine[] | null, target?: readonly VertebraPose[], riding?: boolean): void; point(id:string): [number,number,number] | null; reach(end:'upper'|'lower'): [number,number,number][]; loadTo(parts: readonly PartResult[] | null): (p: number) => void; remove(): void }
type V3 = readonly [number, number, number];
/** How a rod's line is laid out for drawing: kept until its supplied points or the poses they belong to change. */
interface RodPlan { source: readonly V3[]; reference: readonly VertebraPose[]; knots: number[]; blend: { a: number; b: number; t: number }[]; seatAt: number[]; curve: CatmullRomCurve3; centres: Vector3[]; positions: Float32Array; normals: Float32Array }
interface RodShape { key: string; objects: Group['children']; spans: { id: string; low: number; high: number; geometry: BufferGeometry }[]; caps: Mesh[] }
const R = PLACE_DRAWING.rodSegments;
/** Index of a rod tube of `rings` rings, wound outward. */
function tubeGeometry(rings: number): BufferGeometry {
  const geometry = new BufferGeometry(), index: number[] = [];
  geometry.setAttribute('position', new BufferAttribute(new Float32Array(rings * R * 3), 3)); geometry.setAttribute('normal', new BufferAttribute(new Float32Array(rings * R * 3), 3));
  for (let i = 0; i + 1 < rings; i++) for (let j = 0; j < R; j++) { const a = i * R + j, b = i * R + (j + 1) % R; index.push(a, b, a + R, b, b + R, a + R); }
  geometry.setIndex(index); return geometry;
}
/** Rings round each centre, carried along the line so they never twist. */
function ringsAround(centres: readonly Vector3[], radius: number, positions: Float32Array, normals: Float32Array): void {
  const tangent = new Vector3(), normal = new Vector3(), side = new Vector3(), out = new Vector3();
  for (const [i, centre] of centres.entries()) {
    tangent.subVectors(centres[Math.min(centres.length - 1, i + 1)] ?? centre, centres[Math.max(0, i - 1)] ?? centre).normalize();
    if (i === 0) normal.set(Math.abs(tangent.x) < .9 ? 1 : 0, Math.abs(tangent.x) < .9 ? 0 : 1, 0);
    normal.addScaledVector(tangent, -normal.dot(tangent)).normalize(); side.crossVectors(tangent, normal);
    for (let j = 0; j < R; j++) {
      const angle = 2 * Math.PI * j / R, k = (i * R + j) * 3; out.copy(normal).multiplyScalar(Math.cos(angle)).addScaledVector(side, Math.sin(angle));
      normals[k] = out.x; normals[k + 1] = out.y; normals[k + 2] = out.z;
      positions[k] = centre.x + radius * out.x; positions[k + 1] = centre.y + radius * out.y; positions[k + 2] = centre.z + radius * out.z;
    }
  }
}
function middleOf(points: readonly Vector3[], low = 0, high = points.length - 1): [number, number, number] | null {
  let length = 0; for (let i = low + 1; i <= high; i++) { const a = points[i - 1], b = points[i]; if (a !== undefined && b !== undefined) length += a.distanceTo(b); }
  let left = length / 2;
  for (let i = low + 1; i <= high; i++) { const a = points[i - 1], b = points[i]; if (a === undefined || b === undefined) continue; const d = a.distanceTo(b); if (left <= d) return a.clone().lerp(b, d === 0 ? 0 : left / d).toArray(); left -= d; }
  return points[low]?.toArray() ?? null;
}
export function drawPlacement(spine: Pick<DrawnSpine, 'levels' | 'addLayer'>, fit: SpineFit, layout: ImplantLayout, unsettled: readonly string[]): DrawnPlacement {
  const layer = spine.addLayer(), geometries: BufferGeometry[] = [], parts: Group[] = [];
  const own = <T extends BufferGeometry>(geometry: T): T => { geometries.push(geometry); return geometry; };
  const inks = new Map<string, ReturnType<SceneLayer['flat']> | ReturnType<SceneLayer['lit']>>();
  /** Metal for screws, hooks and rods; flat ink for lines and empty places, which carry no light. */
  const ink = (id: string, flat = false) => { let found=inks.get(id); if(found===undefined){if(flat)found=layer.flat(FORMATION.ink,PLACE_DRAWING.ink);else{const metal=layer.lit(PLACE_DRAWING.metal.colour);metal.roughness=PLACE_DRAWING.metal.roughness;metal.metalness=PLACE_DRAWING.metal.metalness;metal.opacity=PLACE_DRAWING.ink;found=metal;}inks.set(id,found);} return found; };
  const rodGroups = new Map<string, Group>(), rodShapes = new Map<string, RodShape>(), rodPlans = new Map<string, RodPlan>(), rodPoints=new Map<string,[number,number,number]>();
  const fitPoses = fit.vertebrae.map((v) => v.pose);
  let posed: readonly VertebraPose[] = fitPoses;
  const add = (group: Group, id: string, parent: Group) => { parent.add(group); layer.pointable(group, id); if (id === 'upper') parts.unshift(group); else if (id === 'lower') parts.splice(1, 0, group); else parts.push(group); };
  for (const [index, anchor] of layout.anchors.entries()) {
    const vertebra = fit.vertebrae.find((v) => v.level === anchor.level);
    if (vertebra === undefined || !spine.levels.includes(anchor.level)) continue;
    const group = new Group(), rotation = new Vector3(...vertebra.pose.rotation), angle = rotation.length();
    const inverse = angle === 0 ? new Quaternion() : new Quaternion().setFromAxisAngle(rotation.normalize(), angle).invert();
    group.position.copy(new Vector3(...anchor.entryMm).sub(new Vector3(...vertebra.pose.centre)).applyQuaternion(inverse));
    const direction = new Vector3(...anchor.directionUnit).applyQuaternion(inverse);
    // The group's y runs along the supplied direction at its supplied length, so the seat is exactly the layout's: offset times that vector.
    const along = direction.length(); group.quaternion.setFromUnitVectors(new Vector3(0, 1, 0), direction.normalize());
    const anchorInk=ink(`anchor:${String(index)}`,anchor.kind==='none'), rod = layout.rods.find((r) => r.side === anchor.side);
    const seat = -(rod?.headOffsetMm ?? 0) * along, cupRadius = rod === undefined ? PLACE_DRAWING.headRadius : rod.diameterMm * PLACE_DRAWING.headOverRod / 2, cupDepth = rod === undefined ? PLACE_DRAWING.headDepth : rod.diameterMm * PLACE_DRAWING.headOverRod;
    const cup = () => { const head = new Mesh(own(new CylinderGeometry(cupRadius, cupRadius, cupDepth, PLACE_DRAWING.screwSegments)), anchorInk); head.position.y = seat; return head; };
    /** A neck from under the head down to `to`, so the shank or the hook's body meets its head without crossing it. */
    const neck = (radius: number, to: number) => { const from = seat + cupDepth / 2; if (to <= from) return null; const part = new Mesh(own(new CylinderGeometry(radius, radius, to - from, PLACE_DRAWING.screwSegments)), anchorInk); part.position.y = (from + to) / 2; return part; };
    if (anchor.kind === 'screw') {
      const shaft = anchor.diameterMm !== null && anchor.lengthMm !== null ? neck(anchor.diameterMm / 2, anchor.lengthMm) : null;
      if (shaft !== null) group.add(shaft);
      group.add(cup());
    } else {
      const ring = new Mesh(own(new TorusGeometry(PLACE_DRAWING.hookRadius, anchor.kind === 'hook' ? PLACE_DRAWING.hookWidth : PLACE_DRAWING.emptyWidth, 6, 24, anchor.kind === 'hook' ? Math.PI * 1.5 : Math.PI * 2)), anchorInk);
      ring.rotation.x = Math.PI / 2; group.add(ring);
      if (anchor.kind === 'hook' && rod !== undefined) { group.add(cup()); const body = neck(rod.diameterMm / 2, 0); if (body !== null) group.add(body); }
    }
    add(group, `anchor:${String(index)}`, layer.onBone(anchor.level));
  }
  for (const [upper, level] of [[true, layout.upper], [false, layout.lower]] as const) {
    const v = fit.vertebrae.find((v) => v.level === level); if (v === undefined || !spine.levels.includes(level)) continue;
    const group = new Group(), width = upper ? v.pose.topHalfWidth : v.pose.bottomHalfWidth, z = v.pose.height / 2 * (upper ? 1 : -1);
    const pieces = unsettled.includes(level) ? 8 : 1;
    for (let i = 0; i < pieces; i++) {
      const start = -width + 2 * width * i / pieces, end = pieces === 1 ? width : start + width / pieces;
      group.add(new Line(own(new BufferGeometry().setFromPoints([new Vector3(0, start, z), new Vector3(0, end, z)])), ink(upper?'upper':'lower',true)));
    }
    add(group, upper ? 'upper' : 'lower', layer.onBone(level));
  }
  const clearRod = (side: string) => {
    const shape = rodShapes.get(side); if (shape === undefined) return;
    for (const object of shape.objects) layer.unpointable(object);
    for (const span of shape.spans) { span.geometry.dispose(); rodPoints.delete(span.id); } for (const cap of shape.caps) cap.geometry.dispose();
    rodGroups.get(side)?.clear(); rodShapes.delete(side);
  };
  /** Seats found on the supplied line, the knots kept for the curve, and how each knot shares the moves of the two seats either side of it. */
  const planOf = (rod: RodProposal, anchors: readonly AnchorProposal[], source: readonly V3[], reference: readonly VertebraPose[]): RodPlan => {
    const known = rodPlans.get(rod.side); if (known?.source === source && known.reference === reference) return known;
    const distance = (a: V3, b: V3) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    const seatIndex = anchors.map((a) => { const seat = seatPoint(a, rod.headOffsetMm, fit, reference); let best = 0; source.forEach((p, i) => { if (distance(p, seat) < distance(source[best] ?? p, seat)) best = i; }); return best; });
    const must = new Set([0, source.length - 1, ...seatIndex]), knots: number[] = [];
    for (const [i, point] of source.entries()) {
      const last = knots.at(-1), gap = last === undefined ? Infinity : distance(point, source[last] ?? point);
      if (last === undefined || gap >= PLACE_DRAWING.knotStepsMm) knots.push(i);
      else if (must.has(i) && gap > 1e-6) { if (must.has(last)) knots.push(i); else knots[knots.length - 1] = i; }
    }
    const knotOf = (index: number) => knots.reduce((best, k, i) => Math.abs(k - index) < Math.abs((knots[best] ?? k) - index) ? i : best, 0);
    const seatAt = seatIndex.map(knotOf), along = [0];
    for (let k = 1; k < knots.length; k++) along.push((along[k - 1] ?? 0) + distance(source[knots[k] ?? 0] ?? [0, 0, 0], source[knots[k - 1] ?? 0] ?? [0, 0, 0]));
    const order = seatAt.map((at, a) => ({ at, a })).sort((x, y) => x.at - y.at);
    const blend = knots.map((_, k) => {
      const after = order.findIndex((s) => s.at >= k), next = order[after], previous = order[after - 1];
      if (next === undefined) { const last = order.at(-1); return { a: last?.a ?? -1, b: last?.a ?? -1, t: 0 }; }
      if (previous === undefined || next.at === k) return { a: next.a, b: next.a, t: 0 };
      const from = along[previous.at] ?? 0, to = along[next.at] ?? 0;
      return { a: previous.a, b: next.a, t: to === from ? 0 : ((along[k] ?? 0) - from) / (to - from) };
    });
    const curve = new CatmullRomCurve3(knots.map(() => new Vector3()), false, 'centripetal'), rings = (knots.length - 1) * PLACE_DRAWING.rodSteps + 1;
    const plan = { source, reference, knots, blend, seatAt, curve, centres: Array.from({ length: rings }, () => new Vector3()), positions: new Float32Array(rings * R * 3), normals: new Float32Array(rings * R * 3) };
    rodPlans.set(rod.side, plan); return plan;
  };
  const scratch = new Vector3(), other = new Vector3();
  const drawRods = (poses: readonly VertebraPose[], loaded: boolean, lines: RodLine[] | null, target?: readonly VertebraPose[], riding=false) => {
    posed = poses;
    for (const rod of layout.rods) {
      const anchors=layout.anchors.filter((a)=>a.side===rod.side && a.kind!=='none');
      const seated=loaded?lines?.find((l)=>l.side===rod.side)?.pointsMm:undefined, moving=loaded||riding;
      // The seated line belongs to the bench's poses, the standing line to the fit's; each point then rides its two nearest seats' bones.
      const [source,reference]:[readonly V3[]|undefined,readonly VertebraPose[]]=seated!==undefined&&seated.length>1?[seated,target??poses]:rod.standingPointsMm.length>1||!moving?[rod.standingPointsMm,fitPoses]:[anchors.map((a)=>seatPoint(a,rod.headOffsetMm,fit,poses)),poses];
      if(source.length<2){clearRod(rod.side);rodPoints.delete(`rod:${rod.side}`);continue;}
      const plan=planOf(rod,anchors,source,reference), still=!moving||reference===poses;
      const motions=anchors.map((a):Matrix4|null=>{ if(still)return null; const from=reference.find((p)=>p.level===a.level),to=poses.find((p)=>p.level===a.level); return from===undefined||to===undefined?null:poseMotion(from,to); });
      for(const [k,index] of plan.knots.entries()){
        const p=source[index],knot=plan.curve.points[k],b=plan.blend[k];if(p===undefined||knot===undefined||b===undefined)continue;
        const ma=motions[b.a]??null,mb=motions[b.b]??null;
        scratch.set(...p);if(ma!==null)scratch.applyMatrix4(ma);
        if(b.t>0){other.set(...p);if(mb!==null)other.applyMatrix4(mb);scratch.lerp(other,b.t);}
        knot.copy(scratch);
      }
      const last=plan.centres.length-1;
      for(const [i,centre] of plan.centres.entries())plan.curve.getPoint(last===0?0:i/last,centre);
      ringsAround(plan.centres,rod.diameterMm/2,plan.positions,plan.normals);
      const spanned=loaded&&anchors.length>=2, steps=PLACE_DRAWING.rodSteps;
      const ranges=spanned?anchors.slice(1).map((to,i)=>{
        const from=anchors[i]??to,a=(plan.seatAt[i]??0)*steps,b=(plan.seatAt[i+1]??0)*steps;
        // The supplied line can continue beyond the outer heads. Keep that overhang.
        return {id:`span:${rod.side}:${from.level}:${to.level}`,low:i===0?0:Math.min(a,b),high:i===anchors.length-2?last:Math.max(a,b)};
      }).filter((r)=>r.high>r.low):[{id:`rod:${rod.side}`,low:0,high:last}];
      const key=ranges.map((r)=>`${r.id}:${String(r.low)}:${String(r.high)}`).join();
      let group=rodGroups.get(rod.side);
      if(group===undefined) { group=new Group();rodGroups.set(rod.side,group);layer.onSpine.add(group);parts.push(group); }
      let shape=rodShapes.get(rod.side);
      if(shape?.key!==key){
        clearRod(rod.side);
        const made:RodShape={key,objects:[],spans:[],caps:[]};
        for(const range of ranges){
          const geometry=tubeGeometry(range.high-range.low+1),segment=new Group();segment.add(new Mesh(geometry,ink(range.id)));
          group.add(segment);layer.pointable(segment,range.id);made.objects.push(segment);made.spans.push({...range,geometry});
        }
        for(const id of [ranges[0]?.id,ranges.at(-1)?.id]){
          const cap=new Mesh(new SphereGeometry(rod.diameterMm/2,R,R/2),ink(id??`rod:${rod.side}`));group.add(cap);layer.pointable(cap,`rod:${rod.side}`);made.objects.push(cap);made.caps.push(cap);
        }
        rodShapes.set(rod.side,made);shape=made;
      }
      for(const span of shape.spans){
        const position=span.geometry.getAttribute('position') as BufferAttribute,normal=span.geometry.getAttribute('normal') as BufferAttribute;
        (position.array as Float32Array).set(plan.positions.subarray(span.low*R*3,(span.high+1)*R*3));(normal.array as Float32Array).set(plan.normals.subarray(span.low*R*3,(span.high+1)*R*3));
        position.needsUpdate=true;normal.needsUpdate=true;span.geometry.computeBoundingSphere();
        const middle=middleOf(plan.centres,span.low,span.high);if(middle!==null)rodPoints.set(span.id,middle);
      }
      shape.caps[0]?.position.copy(plan.centres[0]??scratch);shape.caps[1]?.position.copy(plan.centres[last]??scratch);
      const middle=middleOf(plan.centres);if(middle!==null)rodPoints.set(`rod:${rod.side}`,middle);
    }
    layer.changed();
  };
  drawRods(fitPoses,false,null);
  layer.changed();
  return { layer, parts,
    poses: drawRods,
    point: (id)=>rodPoints.get(id)??null,
    /** Where the hardware an end's name could lie on stands now: the entries and heads at its level, and every point along the drawn rods. */
    reach(end) {
      const level=layout[end],at=layout.anchors.filter((a)=>a.level===level&&a.kind!=='none'&&spine.levels.includes(level)).flatMap((a)=>{const rod=layout.rods.find((r)=>r.side===a.side);return rod===undefined?[anchorPoint(a,fit,posed)]:[anchorPoint(a,fit,posed),seatPoint(a,rod.headOffsetMm,fit,posed)];});
      return [...at,...[...rodShapes.keys()].flatMap((side)=>rodPlans.get(side)?.centres.map((c)=>c.toArray())??[])];
    },
    loadTo(loads) {
      const transitions=[...inks].map(([id,material])=>{
        const anchor=id.startsWith('anchor:')?layout.anchors[Number(id.slice(7))]:undefined;
        const part=anchor?.kind==='none'?undefined:anchor===undefined?loads?.find((p)=>p.part.kind==='rod-span'?id===`span:${p.part.side}:${p.part.from}:${p.part.to}`:p.part.kind==='rod'&&id===`rod:${p.part.side}`):anchorLoad(loads??[],anchor);
        const share=part?.share.p50??0;
        return {material,from:material.opacity,to:part===undefined?PLACE_DRAWING.ink:.25+.75*Math.max(0,Math.min(1,share))};
      });
      return (p)=>{for(const t of transitions)t.material.opacity=t.from+(t.to-t.from)*p;layer.changed();};
    },
    reveal(count, strength) { parts.forEach((part, i) => { part.visible = i < count; });for(const material of inks.values())material.opacity=PLACE_DRAWING.ink*strength; layer.changed(); },
    compareInk(changed) { for(const [id,material] of inks)material.opacity=changed.includes(id)||id.startsWith('span:')&&changed.includes(`rod:${id.split(':')[1]??''}`)?1:.4;layer.changed(); },
    fadeParts(ids,progress) {for(const [id,material] of inks)if(ids.includes(id)||id.startsWith('span:')&&ids.includes(`rod:${id.split(':')[1]??''}`))material.opacity=PLACE_DRAWING.ink*progress;layer.changed();},
    remove() { layer.remove(); for (const geometry of geometries) geometry.dispose(); for (const side of [...rodShapes.keys()]) { const shape = rodShapes.get(side); for (const span of shape?.spans ?? []) span.geometry.dispose(); for (const cap of shape?.caps ?? []) cap.geometry.dispose(); } rodShapes.clear(); }
  };
}
