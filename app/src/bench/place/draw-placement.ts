import { BufferGeometry, CurvePath, CylinderGeometry, Group, Line, LineCurve3, Mesh, Quaternion, SphereGeometry, TorusGeometry, TubeGeometry, Vector3 } from 'three';
import type { ImplantLayout, SpineFit, PartResult, RodLine, VertebraPose } from '@/shapes';
import type { DrawnSpine } from '@/spine-3d/scene/drawn-spine';
import type { SceneLayer } from '@/spine-3d/scene/scene-layer';
import { anchorLoad, anchorPoint, movedPoint } from './load-view';
import { FORMATION } from '@/spine-3d/scene/look';

/** Symbolic heads and hooks; only screw shafts and rods use supplied dimensions. */
export const PLACE_DRAWING = { bone: .55, reviewBone: .34, ink: .8, headRadius: 3.2, headDepth: 2, hookRadius: 4, hookWidth: .7, emptyWidth: .25, screwSegments: 10, ringRadiusPx: 10, ringWidthPx: 1.5, staggerMs: 60, arrivalMs: 1200, backAzimuth: 146, backElevation: 14, endHandleRadiusPx: 12 } as const;
export interface DrawnPlacement { layer: SceneLayer; parts: Group[]; reveal(count: number, strength: number): void; fadeParts(ids:readonly string[],progress:number):void; compareInk(changed:readonly string[]):void; poses(poses: readonly VertebraPose[], loaded: boolean, rods: RodLine[] | null, target?: readonly VertebraPose[], riding?: boolean): void; point(id:string): [number,number,number] | null; loadTo(parts: readonly PartResult[] | null): (p: number) => void; remove(): void }
export function drawPlacement(spine: Pick<DrawnSpine, 'levels' | 'addLayer'>, fit: SpineFit, layout: ImplantLayout, unsettled: readonly string[]): DrawnPlacement {
  const layer = spine.addLayer(), geometries: BufferGeometry[] = [], parts: Group[] = [];
  const own = <T extends BufferGeometry>(geometry: T): T => { geometries.push(geometry); return geometry; };
  const inks = new Map<string, ReturnType<SceneLayer['flat']>>();
  const ink = (id: string) => { let found=inks.get(id); if(found===undefined){found=layer.flat(FORMATION.ink,PLACE_DRAWING.ink);inks.set(id,found);} return found; };
  const rodGroups = new Map<string, Group>();
  let rodGeometry: BufferGeometry[] = [];
  const rodObjects: Group['children'] = [], rodPoints=new Map<string,[number,number,number]>();
  const middleOf=(points:readonly [number,number,number][]):[number,number,number]|null=>{
    let length=0;const lengths=points.slice(1).map((point,i)=>{const d=new Vector3(...point).distanceTo(new Vector3(...(points[i]??point)));length+=d;return d;});
    let left=length/2;for(const [i,d] of lengths.entries()){const a=points[i],b=points[i+1];if(a===undefined||b===undefined)continue;if(left<=d)return new Vector3(...a).lerp(new Vector3(...b),d===0?0:left/d).toArray();left-=d;}return points[0]??null;
  };
  const rodOwn = <T extends BufferGeometry>(geometry:T):T => { rodGeometry.push(geometry);return geometry; };
  const add = (group: Group, id: string, parent: Group) => { parent.add(group); layer.pointable(group, id); if (id === 'upper') parts.unshift(group); else if (id === 'lower') parts.splice(1, 0, group); else parts.push(group); };
  for (const [index, anchor] of layout.anchors.entries()) {
    const vertebra = fit.vertebrae.find((v) => v.level === anchor.level);
    if (vertebra === undefined || !spine.levels.includes(anchor.level)) continue;
    const group = new Group(), rotation = new Vector3(...vertebra.pose.rotation), angle = rotation.length();
    const inverse = angle === 0 ? new Quaternion() : new Quaternion().setFromAxisAngle(rotation.normalize(), angle).invert();
    group.position.copy(new Vector3(...anchor.entryMm).sub(new Vector3(...vertebra.pose.centre)).applyQuaternion(inverse));
    const direction = new Vector3(...anchor.directionUnit).applyQuaternion(inverse).normalize();
    group.quaternion.setFromUnitVectors(new Vector3(0, 1, 0), direction);
    const anchorInk=ink(`anchor:${String(index)}`);
    if (anchor.kind === 'screw') {
      if (anchor.diameterMm !== null && anchor.lengthMm !== null) {
        const shaft = new Mesh(own(new CylinderGeometry(anchor.diameterMm / 2, anchor.diameterMm / 2, anchor.lengthMm, PLACE_DRAWING.screwSegments)), anchorInk);
        shaft.position.y = anchor.lengthMm / 2; group.add(shaft);
      }
      const head = new Mesh(own(new CylinderGeometry(PLACE_DRAWING.headRadius, PLACE_DRAWING.headRadius, PLACE_DRAWING.headDepth, PLACE_DRAWING.screwSegments)), anchorInk); group.add(head);
    } else {
      const ring = new Mesh(own(new TorusGeometry(PLACE_DRAWING.hookRadius, anchor.kind === 'hook' ? PLACE_DRAWING.hookWidth : PLACE_DRAWING.emptyWidth, 6, 24, anchor.kind === 'hook' ? Math.PI * 1.5 : Math.PI * 2)), anchorInk);
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
      group.add(new Line(own(new BufferGeometry().setFromPoints([new Vector3(0, start, z), new Vector3(0, end, z)])), ink(upper?'upper':'lower')));
    }
    add(group, upper ? 'upper' : 'lower', layer.onBone(level));
  }
  const drawRods = (poses: readonly VertebraPose[], loaded: boolean, lines: RodLine[] | null, target?: readonly VertebraPose[], riding=false) => {
    for(const object of rodObjects)layer.unpointable(object);rodObjects.length=0;rodPoints.clear();
    for(const geometry of rodGeometry) geometry.dispose(); rodGeometry=[];
    for(const group of rodGroups.values()) group.clear();
    for (const rod of layout.rods) {
      const anchors=layout.anchors.filter((a)=>a.side===rod.side && a.kind!=='none');
      const seated=lines?.find((l)=>l.side===rod.side)?.pointsMm.map((point)=>{
        if(target===undefined)return point;
        const nearest=anchors.reduce<(typeof anchors)[number]|null>((best,a)=>best===null||new Vector3(...anchorPoint(a,fit,target)).distanceToSquared(new Vector3(...point))<new Vector3(...anchorPoint(best,fit,target)).distanceToSquared(new Vector3(...point))?a:best,null);
        const from=target.find((p)=>p.level===nearest?.level),to=poses.find((p)=>p.level===nearest?.level);
        return from===undefined||to===undefined?point:movedPoint(point,from,to);
      });
      const points=loaded ? seated ?? anchors.map((a)=>anchorPoint(a,fit,poses)) : riding ? anchors.map((a)=>anchorPoint(a,fit,poses)) : rod.standingPointsMm;
      if(points.length<2) continue;
      const middle=middleOf(points);if(middle!==null)rodPoints.set(`rod:${rod.side}`,middle);
      let group=rodGroups.get(rod.side);
      if(group===undefined) { group=new Group();rodGroups.set(rod.side,group);layer.onSpine.add(group);parts.push(group); }
      const addLine=(points:readonly [number,number,number][],id:string)=>{
        if(points.length<2)return;
        const path = new CurvePath<Vector3>();
        for(let i=1;i<points.length;i++){const a=points[i-1],b=points[i];if(a!==undefined&&b!==undefined)path.add(new LineCurve3(new Vector3(...a),new Vector3(...b)));}
        const segment=new Group();segment.add(new Mesh(rodOwn(new TubeGeometry(path,Math.max(1,points.length-1),rod.diameterMm/2,PLACE_DRAWING.screwSegments,false)),ink(id)));
        group.add(segment);layer.pointable(segment,id);rodObjects.push(segment);const middle=middleOf(points);if(middle!==null)rodPoints.set(id,middle);
      };
      if(!loaded || anchors.length<2) addLine(points,`rod:${rod.side}`);
      else for(let i=1;i<anchors.length;i++){
        const from=anchors[i-1],to=anchors[i];if(from===undefined||to===undefined)continue;
        const a=anchorPoint(from,fit,poses),b=anchorPoint(to,fit,poses);
        const nearest=(point:readonly number[])=>points.reduce((best,p,j)=>new Vector3(...p).distanceToSquared(new Vector3(...point))<new Vector3(...(points[best]??p)).distanceToSquared(new Vector3(...point))?j:best,0);
        const ai=nearest(a),bi=nearest(b),low=i===1?0:Math.min(ai,bi),high=i===anchors.length-1?points.length-1:Math.max(ai,bi);
        // The supplied line can continue beyond the outer heads. Keep that overhang.
        const line=seated===undefined?[a,b]:points.slice(low,high+1);
        addLine(line,`span:${rod.side}:${from.level}:${to.level}`);
      }
      for (const [i,at] of [points[0],points.at(-1)].entries()) if(at!==undefined) {
        const from=anchors[i===0?0:anchors.length-2],to=anchors[i===0?1:anchors.length-1];
        const id=loaded&&from!==undefined&&to!==undefined?`span:${rod.side}:${from.level}:${to.level}`:`rod:${rod.side}`;
        const cap=new Mesh(rodOwn(new SphereGeometry(rod.diameterMm/2,10,6)),ink(id));cap.position.set(...at);group.add(cap);layer.pointable(cap,`rod:${rod.side}`);rodObjects.push(cap);
      }
    }
    layer.changed();
  };
  drawRods(fit.vertebrae.map((v)=>v.pose),false,null);
  layer.changed();
  return { layer, parts,
    poses: drawRods,
    point: (id)=>rodPoints.get(id)??null,
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
    remove() { layer.remove(); for (const geometry of [...geometries,...rodGeometry]) geometry.dispose(); }
  };
}
